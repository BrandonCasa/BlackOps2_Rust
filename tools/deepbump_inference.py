"""Original DeepBump ONNX inference with cooperative CPU/CUDA ownership."""
import gc
import os
import subprocess

import numpy as np

from preparation_resources import PreparationResources


class DeepBumpSession:
    def __init__(self, weights, pause_file=None, device="cpu", saturate=True, cuda_graph=True):
        import onnxruntime as ort
        if device not in ("cpu", "cuda"):
            raise ValueError("DeepBump preparation supports CPU or CUDA")
        ort.disable_telemetry_events()
        self.ort, self.weights, self.device = ort, weights, device
        self.options = ort.SessionOptions()
        self.options.intra_op_num_threads = (os.cpu_count() or 1) if saturate else 1
        self.options.inter_op_num_threads = 1
        self.resources = PreparationResources(pause_file, gpu_memory_mib=5120,
                                              exit_for_shared_slot=True, saturate=saturate)
        self.cuda_graph = cuda_graph
        self.binding = self.input_value = self.output_value = None
        self.session = None
        self.binding = self.input_value = self.output_value = None

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
                free = int(subprocess.check_output(["nvidia-smi", "--id=0", "--query-gpu=memory.free",
                    "--format=csv,noheader,nounits"], text=True).strip())*1024**2
                budget = max(1024**3, free-1024**3) if self.resources.saturate else 4*1024**3
                providers = [("CUDAExecutionProvider", {
                    "use_tf32": "0", "gpu_mem_limit": str(budget),
                    "arena_extend_strategy": "kSameAsRequested",
                    "cudnn_conv_use_max_workspace": "1", "cudnn_conv_algo_search": "EXHAUSTIVE",
                    "enable_cuda_graph": "1" if self.cuda_graph else "0",
                })]
            else:
                providers = ["CPUExecutionProvider"]
            self.session = self.ort.InferenceSession(str(self.weights), self.options, providers=providers)
            if gpu and self.session.get_providers()[0] != "CUDAExecutionProvider":
                raise ValueError("CUDA initialization failed; refusing an implicit device fallback")
            self.session.disable_fallback()

    def run(self, tensor):
        self.ready()
        if self.device == "cuda" and self.cuda_graph:
            if self.binding is None:
                self.input_value = self.ort.OrtValue.ortvalue_from_numpy(tensor, "cuda", 0)
                self.output_value = self.ort.OrtValue.ortvalue_from_shape_and_type((1, 3, 1024, 1024), np.float32, "cuda", 0)
                self.binding = self.session.io_binding()
                self.binding.bind_ortvalue_input(self.session.get_inputs()[0].name, self.input_value)
                self.binding.bind_ortvalue_output(self.session.get_outputs()[0].name, self.output_value)
            else:
                self.input_value.update_inplace(tensor)
            self.session.run_with_iobinding(self.binding)
            return self.binding.copy_outputs_to_cpu()[0]
        return self.session.run(None, {self.session.get_inputs()[0].name: tensor})[0]
