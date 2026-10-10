#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include <cstdint>
#include <fstream>
#include <iomanip>
#include <stdexcept>

ContentPrinter::ContentPrinter(const Zone& zone) : m_zone(zone) {}
void ContentPrinter::PrintContent() const {
    using namespace T6;
    std::ofstream metadata("model-lod-frames.json");
    std::ofstream binary("model-lod-frames.bin", std::ios::binary);
    metadata << '{';
    bool first_model = true;
    for (auto* asset : m_zone.m_pools) {
        if (asset->m_type != ASSET_TYPE_XMODEL || asset->IsReference()) continue;
        const auto* model = static_cast<const XModel*>(asset->m_ptr);
        if (!first_model) metadata << ',';
        first_model = false;
        metadata << std::quoted(model->name) << ":[";
        for (unsigned level = 0; level < model->numLods; ++level) {
            if (level) metadata << ',';
            const auto& lod = model->lodInfo[level];
            metadata << "{\"distance\":" << lod.dist << ",\"surfaces\":[";
            for (unsigned s = 0; s < lod.numsurfs; ++s) {
                if (s) metadata << ',';
                const unsigned index = lod.surfIndex + s;
                const auto& surface = model->surfs[index];
                const auto* material = model->materialHandles[index];
                if (!material || (surface.vertCount && !surface.verts0) ||
                    (surface.triCount && !surface.triIndices))
                    throw std::runtime_error("Missing authored model surface data");
                metadata << "{\"material\":" << std::quoted(material->info.name)
                         << ",\"vertices\":" << surface.vertCount
                         << ",\"triangles\":" << surface.triCount
                         << ",\"vertex_offset\":" << binary.tellp();
                for (unsigned v = 0; v < surface.vertCount; ++v) {
                    const auto& vertex = surface.verts0[v];
                    const uint32_t tangent = vertex.tangent.packed;
                    const float sign = vertex.binormalSign;
                    const uint32_t color = vertex.color.packed;
                    const uint32_t normal = vertex.normal.packed;
                    binary.write(reinterpret_cast<const char*>(&tangent), sizeof(tangent));
                    binary.write(reinterpret_cast<const char*>(&sign), sizeof(sign));
                    binary.write(reinterpret_cast<const char*>(&color), sizeof(color));
                    binary.write(reinterpret_cast<const char*>(&normal), sizeof(normal));
                    binary.write(reinterpret_cast<const char*>(&vertex.xyz), sizeof(vertex.xyz));
                }
                metadata << ",\"index_offset\":" << binary.tellp() << '}';
                for (unsigned t = 0; t < surface.triCount; ++t)
                    for (unsigned k = 0; k < 3; ++k) {
                        const uint16_t value = surface.triIndices[t].i[k];
                        binary.write(reinterpret_cast<const char*>(&value), sizeof(value));
                    }
            }
            metadata << "]}";
        }
        metadata << ']';
    }
    metadata << '}';
    if (!metadata.good() || !binary.good()) throw std::runtime_error("Model LOD frame export failed");
}
