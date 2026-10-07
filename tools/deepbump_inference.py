"""Original DeepBump ONNX inference with cooperative CPU/CUDA ownership."""
import gc

from preparation_resources import PreparationResources


class DeepBumpSession:
    def __init__(self, weights, pause_file=None, device="cpu"):
        import onnxruntime as ort
        if device not in ("cpu", "cuda"):
            raise ValueError("DeepBump preparation supports CPU or CUDA")
        ort.disable_telemetry_events()
        self.ort, self.weights, self.device = ort, weights, device
        self.options = ort.SessionOptions()
        self.options.intra_op_num_threads = 1
        self.options.inter_op_num_threads = 1
        self.resources = PreparationResources(pause_file, gpu_memory_mib=5120,
                                              exit_for_shared_slot=True)
        self.session = None

    def offload(self):
        # Destroy the provider session/arena rather than retaining its CPU/GPU
        # allocations while another task owns the game or profiling workload.
        self.session = None
        gc.collect()

    def ready(self):
        gpu = self.device == "cuda"
        owned = gpu and self.session is not None
        if self.resources.refresh(gpu_admission=gpu and not owned, gpu_owned=owned):
            self.resources.wait(gpu_admission=gpu and not owned, gpu_owned=owned,
                                on_pause=self.offload)
        if self.session is None:
            self.resources.wait(gpu_admission=gpu)
            if gpu:
                self.ort.preload_dlls()
                if "CUDAExecutionProvider" not in self.ort.get_available_providers():
                    raise ValueError("Use a CUDA-enabled ONNX Runtime matching the installed CUDA runtime")
                providers = [("CUDAExecutionProvider", {
                    "use_tf32": "0", "gpu_mem_limit": str(4*1024**3),
                    "arena_extend_strategy": "kSameAsRequested",
                })]
            else:
                providers = ["CPUExecutionProvider"]
            self.session = self.ort.InferenceSession(str(self.weights), self.options, providers=providers)
            if gpu and self.session.get_providers()[0] != "CUDAExecutionProvider":
                raise ValueError("CUDA initialization failed; refusing an implicit device fallback")
            self.session.disable_fallback()

    def run(self, tensor):
        self.ready()
        return self.session.run(None, {self.session.get_inputs()[0].name: tensor})[0]
