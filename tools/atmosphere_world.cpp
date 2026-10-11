#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include <cmath>
#include <fstream>
#include <iomanip>
#include <stdexcept>

ContentPrinter::ContentPrinter(const Zone& zone):m_zone(zone) {}

void ContentPrinter::PrintContent() const {
    using namespace T6;
    const GfxWorld* world = nullptr;
    for (auto* asset : m_zone.m_pools) {
        if (asset->m_type == ASSET_TYPE_GFXWORLD && !asset->IsReference()) {
            if (world) throw std::runtime_error("multiple worlds");
            world = static_cast<const GfxWorld*>(asset->m_ptr);
        }
    }
    if (!world || world->worldFogVolumeCount > 4096) throw std::runtime_error("invalid world");
    std::ofstream out("atmosphere-world.json");
    out << std::setprecision(9);
    auto vector = [&](const float* values, unsigned count) {
        out << '[';
        for (unsigned i = 0; i < count; ++i) {
            if (!std::isfinite(values[i])) throw std::runtime_error("nonfinite atmosphere value");
            if (i) out << ',';
            out << values[i];
        }
        out << ']';
    };
    auto fog = [&](const GfxWorldFog& value) {
        out << "{\"distance\":"; vector(&value.baseDist, 4);
        out << ",\"sun_angles\":"; vector(&value.sunFogPitch, 4);
        out << ",\"color\":"; vector(value.fogColor.v, 3);
        out << ",\"opacity\":" << value.fogOpacity;
        out << ",\"sun_color\":"; vector(value.sunFogColor.v, 3);
        out << ",\"sun_opacity\":" << value.sunFogOpacity << '}';
    };
    const auto& sun = world->sunParse.initWorldSun[0];
    out << "{\"sun\":{\"control\":" << sun.control << ",\"angles\":"; vector(sun.angles.v, 3);
    out << ",\"ambient\":"; vector(sun.ambientColor.v, 4);
    out << ",\"diffuse\":"; vector(sun.sunCd.v, 4);
    out << ",\"specular\":"; vector(sun.sunCs.v, 4);
    out << ",\"sky\":"; vector(sun.skyColor.v, 4);
    out << ",\"exposure\":" << sun.exposure << "},\"fog_transition\":" << world->sunParse.fogTransitionTime;
    out << ",\"fog\":"; fog(world->sunParse.initWorldFog[0]);
    out << ",\"volumes\":[";
    for (unsigned i = 0; i < world->worldFogVolumeCount; ++i) {
        const auto& volume = world->worldFogVolumes[i];
        if (i) out << ',';
        out << "{\"mins\":"; vector(volume.mins.v, 3);
        out << ",\"maxs\":"; vector(volume.maxs.v, 3);
        out << ",\"control\":" << volume.control << ",\"control_ex\":" << volume.controlEx;
        out << ",\"transition\":" << volume.fogTransitionTime << ",\"fog\":"; fog(volume.volumeWorldFog[0]);
        out << '}';
    }
    out << "],\"planes\":[";
    for (unsigned i = 0; i < world->worldFogVolumePlaneCount; ++i) {
        if (i) out << ',';
        vector(world->worldFogVolumePlanes[i].plane.v, 4);
    }
    out << "]}";
    if (!out.good()) throw std::runtime_error("atmosphere export failed");
}
