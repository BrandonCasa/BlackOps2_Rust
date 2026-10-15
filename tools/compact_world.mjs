// Use the installed converter to keep only each draw's referenced vertices.
import { NodeIO } from '../target/tools/gltf-transform/node_modules/@gltf-transform/core/dist/index.js';
import { compactPrimitive } from '../target/tools/gltf-transform/node_modules/@gltf-transform/functions/dist/index.js';

const io = new NodeIO();
for (const path of process.argv.slice(2)) {
    const document = await io.read(path);
    let before = 0;
    let after = 0;
    for (const mesh of document.getRoot().listMeshes()) {
        for (const primitive of mesh.listPrimitives()) {
            before += primitive.getAttribute('POSITION').getCount();
            compactPrimitive(primitive);
            after += primitive.getAttribute('POSITION').getCount();
        }
    }
    await io.write(path, document);
    console.log(`${path}: ${before} -> ${after} loaded vertices`);
}
