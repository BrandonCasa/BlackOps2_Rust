// Read-only OAT content-list callback. Emits metadata externally, never game code.
#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include "Game/T6/Image/ImageToCommonConverterT6.h"
#include "SearchPath/SearchPathFilesystem.h"
#include "ObjContainer/IPak/IPak.h"
#include "Image/DdsWriter.h"
#include <cstdlib>
#include <filesystem>
#include <unordered_set>
#include <unordered_map>
#include <iostream>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <stdexcept>

ContentPrinter::ContentPrinter(const Zone& zone) : m_zone(zone) {}
void ContentPrinter::PrintContent() const {
    using namespace T6;
    const GfxWorld* world = nullptr;
    for (auto* asset : m_zone.m_pools)
        if (asset->m_type == ASSET_TYPE_GFXWORLD && !asset->IsReference())
            world = static_cast<const GfxWorld*>(asset->m_ptr);
    if ((!world && !std::getenv("ORIGINS_INCLUDE_ACTOR_IMAGES")) ||
        (world && (world->dpvs.smodelCount > 100000 || !world->dpvs.smodelDrawInsts)))
        throw std::runtime_error("missing or invalid world static instances");
    const unsigned instance_count = world ? world->dpvs.smodelCount : 0;
    std::ofstream out("static-instances.json");
    out << std::setprecision(9) << "[";
    std::ofstream surfaces("model-surfaces.json");
    surfaces << std::setprecision(9) << '{';
    std::unordered_set<const XModel*> providers;
    bool first_provider = true;
    for (unsigned i = 0; i < instance_count; ++i) {
        const auto& instance = world->dpvs.smodelDrawInsts[i];
        if (!instance.model || !instance.model->name)
            throw std::runtime_error("missing static model provider");
        if (i) out << ',';
        out << "{\"model\":" << std::quoted(instance.model->name) << ",\"origin\":[";
        for (unsigned j = 0; j < 3; ++j) {
            if (j) out << ',';
            out << instance.placement.origin.v[j];
        }
        out << "],\"axis\":[";
        for (unsigned j = 0; j < 9; ++j) {
            if (j) out << ',';
            const float value = instance.placement.axis[j / 3].v[j % 3];
            if (!std::isfinite(value)) throw std::runtime_error("non-finite placement");
            out << value;
        }
        out << "],\"scale\":" << instance.placement.scale
            << ",\"probe\":" << unsigned(static_cast<unsigned char>(instance.reflectionProbeIndex))
            << ",\"lighting_id\":" << instance.lightingHandle << ",\"lightmap_colors\":[";
        for (unsigned lod = 0; lod < 4; ++lod) {
            if (lod) out << ',';
            const auto& colors = instance.lmapVertexInfo[lod];
            if (colors.numLmapVertexColors && !colors.lmapVertexColors)
                throw std::runtime_error("missing instance lightmap colors");
            out << '[';
            for (unsigned v = 0; v < colors.numLmapVertexColors; ++v) {
                if (v) out << ',';
                out << colors.lmapVertexColors[v];
            }
            out << ']';
        }
        out << "]}";
        const auto* model = instance.model;
        if (providers.insert(model).second) {
            if (!first_provider) surfaces << ',';
            first_provider = false;
            surfaces << std::quoted(model->name) << ":[";
            const auto& lod = model->lodInfo[0];
            for (unsigned j = 0; j < lod.numsurfs; ++j) {
                if (j) surfaces << ',';
                const unsigned index = lod.surfIndex + j;
                const auto& mesh = model->surfs[index];
                const auto* material = model->materialHandles[index];
                if (!mesh.verts0 || !mesh.triIndices || !material)
                    throw std::runtime_error("missing model surface data");
                surfaces << "{\"material\":" << std::quoted(material->info.name)
                         << ",\"family\":" << std::quoted(material->techniqueSet->name)
                         << ",\"states\":[";
                for (unsigned k = 0; k < material->stateBitsCount; ++k) {
                    if (k) surfaces << ',';
                    surfaces << '[' << material->stateBitsTable[k].loadBits.raw[0] << ','
                             << material->stateBitsTable[k].loadBits.raw[1] << ']';
                }
                surfaces << "],\"state_entries\":[";
                for (unsigned k = 0; k < 36; ++k) {
                    if (k) surfaces << ',';
                    surfaces << unsigned(static_cast<unsigned char>(material->stateBitsEntry[k]));
                }
                surfaces << "],\"textures\":[";
                for (unsigned k = 0; k < material->textureCount; ++k) {
                    if (k) surfaces << ',';
                    const auto& binding = material->textureTable[k];
                    if (!binding.image) throw std::runtime_error("missing material image");
                    surfaces << "{\"semantic\":" << unsigned(binding.semantic)
                             << ",\"hash\":" << binding.nameHash
                             << ",\"image\":" << std::quoted(binding.image->name) << '}';
                }
                surfaces << "],\"vertices\":[";
                for (unsigned v = 0; v < mesh.vertCount; ++v) {
                    if (v) surfaces << ',';
                    const auto& vertex = mesh.verts0[v];
                    surfaces << '[' << vertex.tangent.packed << ',' << vertex.binormalSign
                             << ',' << vertex.color.packed << ']';
                }
                surfaces << "],\"indices\":[";
                for (unsigned t = 0; t < mesh.triCount; ++t)
                    for (unsigned k = 0; k < 3; ++k) {
                        if (t || k) surfaces << ',';
                        surfaces << mesh.triIndices[t].i[k];
                    }
                surfaces << "]}";
            }
            surfaces << ']';
        }
    }
    std::unordered_set<const GfxImage*> required;
    if (std::getenv("ORIGINS_INCLUDE_ACTOR_IMAGES")) {
        for (auto* asset : m_zone.m_pools) {
            if (asset->m_type != ASSET_TYPE_XMODEL || asset->IsReference()) continue;
            const auto* model = static_cast<const XModel*>(asset->m_ptr);
            for (unsigned j = 0; j < model->numsurfs; ++j) {
                const auto* material = model->materialHandles[j];
                if (!material) continue;
                for (unsigned k = 0; k < material->textureCount; ++k)
                    if (material->textureTable[k].image)
                        required.insert(material->textureTable[k].image);
            }
        }
    }
    out << ']';
    if (!out.good()) throw std::runtime_error("instance metadata write failed");
    out.close();
    surfaces << '}';
    if (!surfaces.good()) throw std::runtime_error("model surface metadata write failed");
    const char* data_root = std::getenv("ORIGINS_DATA_ROOT");
    const char* output_root = std::getenv("ORIGINS_IMAGE_OUTPUT");
    if (!data_root || !output_root) return;
    SearchPathFilesystem search(data_root);
    for (unsigned i = 0; i < instance_count; ++i) {
        const auto* model = world->dpvs.smodelDrawInsts[i].model;
        for (unsigned j = 0; j < model->numsurfs; ++j) {
            const auto* material = model->materialHandles[j];
            for (unsigned k = 0; k < material->textureCount; ++k)
                if (material->textureTable[k].image)
                    required.insert(material->textureTable[k].image);
        }
    }
    for (const auto* name : {"dlczm4.ipak", "patch_zm.ipak", "dlczm3.ipak", "dlczm2.ipak", "dlczm1.ipak", "dlczm0.ipak", "dlczm0_load_zm.ipak", "zm_nuked_patch.ipak", "zm_nuked.ipak", "zm.ipak", "base.ipak"}) {
        auto file = search.Open(name);
        if (!file.IsOpen()) continue;
        auto ipak = IIPak::Create(name, std::move(file.m_stream));
        if (ipak->Initialize()) IIPak::Repository.AddContainer(std::move(ipak), const_cast<Zone*>(&m_zone));
    }
    for (auto* asset : m_zone.m_pools) {
        if (asset->m_type != ASSET_TYPE_IMAGE || asset->IsReference()) continue;
        const auto* image = static_cast<const GfxImage*>(asset->m_ptr);
        if (!required.contains(image)) continue;
        auto relative = std::filesystem::path(std::string(image->name) + ".dds");
        if (relative.is_absolute()) throw std::runtime_error("absolute image name");
        for (const auto& part : relative)
            if (part == "..") throw std::runtime_error("escaped image name");
        const auto path = std::filesystem::path(output_root) / relative;
        if (std::filesystem::is_regular_file(path)) continue;
        std::cout << "Required image: " << image->name << std::endl;
        auto texture = image::ToCommonConverterT6().Convert(*asset, search);
        image::DdsWriter writer;
        if (!texture || !writer.SupportsImageFormat(texture->GetFormat())) continue;
        std::filesystem::create_directories(path.parent_path());
        std::ofstream file(path, std::ios::binary);
        writer.DumpImage(file, texture.get());
        if (!file.good()) throw std::runtime_error("image export failed");
    }
}
