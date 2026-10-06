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
    if (!world || world->dpvs.smodelCount > 100000 || !world->dpvs.smodelDrawInsts)
        throw std::runtime_error("missing or invalid world static instances");
    std::ofstream out("static-instances.json");
    out << std::setprecision(9) << "[";
    for (unsigned i = 0; i < world->dpvs.smodelCount; ++i) {
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
        out << "],\"scale\":" << instance.placement.scale << '}';
    }
    out << ']';
    if (!out.good()) throw std::runtime_error("instance metadata write failed");
    out.close();
    const char* data_root = std::getenv("ORIGINS_DATA_ROOT");
    const char* output_root = std::getenv("ORIGINS_IMAGE_OUTPUT");
    if (!data_root || !output_root) return;
    SearchPathFilesystem search(data_root);
    std::unordered_set<const GfxImage*> required;
    for (unsigned i = 0; i < world->dpvs.smodelCount; ++i) {
        const auto* model = world->dpvs.smodelDrawInsts[i].model;
        for (unsigned j = 0; j < model->numsurfs; ++j) {
            const auto* material = model->materialHandles[j];
            for (unsigned k = 0; k < material->textureCount; ++k)
                if (material->textureTable[k].semantic == 2 || material->textureTable[k].semantic == 5)
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
