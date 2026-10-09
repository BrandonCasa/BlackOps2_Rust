#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>

ContentPrinter::ContentPrinter(const Zone& zone) : m_zone(zone) {}
void ContentPrinter::PrintContent() const {
    using namespace T6;
    const GfxWorld* world = nullptr;
    const clipMap_t* clip = nullptr;
    for (auto* asset : m_zone.m_pools) {
        if (asset->IsReference()) continue;
        if (asset->m_type == ASSET_TYPE_GFXWORLD) world = static_cast<const GfxWorld*>(asset->m_ptr);
        if (asset->m_type == ASSET_TYPE_CLIPMAP || asset->m_type == ASSET_TYPE_CLIPMAP_PVS)
            clip = static_cast<const clipMap_t*>(asset->m_ptr);
    }
    std::cerr << "brush ownership gfx=" << (world ? world->modelCount : -1)
              << " clip=" << (clip ? clip->numSubModels : 0)
              << " surfaces=" << (world ? world->surfaceCount : -1) << std::endl;
    if (!world || !clip || !world->models || world->modelCount != clip->numSubModels
        || world->modelCount < 2 || world->modelCount > 10000)
        throw std::runtime_error("missing native brush-model ownership");
    std::ofstream out("brush-models.json");
    out << "{\"surfaces\":" << world->surfaceCount << ",\"models\":[";
    for (int i = 0; i < world->modelCount; ++i) {
        const auto& model = world->models[i];
        if (model.surfaceCount && (model.startSurfIndex > static_cast<unsigned>(world->surfaceCount)
            || model.surfaceCount > static_cast<unsigned>(world->surfaceCount) - model.startSurfIndex))
            throw std::runtime_error("invalid native brush-model surface range");
        if (i) out << ',';
        out << "{\"model\":" << i << ",\"first_surface\":" << (model.surfaceCount ? model.startSurfIndex : 0)
            << ",\"surface_count\":" << model.surfaceCount << '}';
    }
    out << "]}";
    if (!out.good()) throw std::runtime_error("brush-model ownership write failed");
}
