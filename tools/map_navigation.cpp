// Read-only compiled BO2 navigation export through existing OAT zone ownership.
#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include <fstream>
#include <iomanip>
#include <cmath>
#include <stdexcept>
ContentPrinter::ContentPrinter(const Zone& zone):m_zone(zone) {}
namespace { void require(bool ok) {if(!ok)throw std::runtime_error("invalid native navigation closure");} }
void ContentPrinter::PrintContent() const {
    using namespace T6;
    const PathData* path=nullptr;
    for(auto* asset:m_zone.m_pools) {
        if(asset->IsReference())continue;
        if(asset->m_type==ASSET_TYPE_GAMEWORLD_SP)path=&static_cast<const GameWorldSp*>(asset->m_ptr)->path;
        if(asset->m_type==ASSET_TYPE_GAMEWORLD_MP)path=&static_cast<const GameWorldMp*>(asset->m_ptr)->path;
    }
    require(path && path->nodes && path->nodeCount>0 && path->nodeCount<=65535);
    std::ofstream out("map-navigation.json");out<<std::setprecision(9)<<"[";
    for(unsigned i=0;i<path->nodeCount;++i) {
        const auto& n=path->nodes[i].constant;
        require(n.totalLinkCount<4096 && (!n.totalLinkCount || n.Links));
        if(i)out<<',';
        out<<"{\"index\":"<<i<<",\"type\":"<<static_cast<int>(n.type)<<",\"position\":[";
        for(unsigned axis=0;axis<3;++axis) {require(std::isfinite(n.vOrigin.v[axis]));if(axis)out<<',';out<<n.vOrigin.v[axis];}
        out<<"],\"animation\":"<<std::quoted(m_zone.m_script_strings.Value(n.animscript))<<",\"links\":[";
        for(unsigned j=0;j<n.totalLinkCount;++j) {
            const auto& link=n.Links[j];require(link.nodeNum<path->nodeCount && std::isfinite(link.fDist) && link.fDist>=0);
            if(j)out<<',';
            out<<"{\"to\":"<<link.nodeNum<<",\"distance\":"<<link.fDist<<",\"negotiation\":"<<static_cast<int>(link.negotiationLink)<<",\"flags\":"<<static_cast<int>(link.flags)<<",\"disconnect\":"<<static_cast<int>(link.disconnectCount)<<'}';
        }
        out<<"]}";
    }
    out<<']';require(out.good());
}
