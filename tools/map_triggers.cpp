// Read-only authored brush-volume export through existing OAT zone ownership.
#include "ContentPrinter.h"
#include "Game/T6/T6_Assets.h"
#include "Game/T6/T6.h"
#include "Pool/XAssetInfo.h"
#include <fstream>
#include <vector>
#include <set>
#include <iostream>
#include <iomanip>
#include <cmath>
#include <stdexcept>
ContentPrinter::ContentPrinter(const Zone& zone):m_zone(zone) {}
namespace {
void require(bool ok) { if (!ok) throw std::runtime_error("invalid native trigger closure"); }
void vector(std::ostream& out,const T6::vec3_t& v,bool extent=false) {
    out << '['; for(unsigned i=0;i<3;++i) { require(std::isfinite(v.v[i]) && (!extent || v.v[i]>=0)); if(i)out<<',';out<<v.v[i]; } out<<']';
}
}
void ContentPrinter::PrintContent() const {
    using namespace T6;

    const clipMap_t* map=nullptr;
    for(auto* asset:m_zone.m_pools) if((asset->m_type==ASSET_TYPE_CLIPMAP || asset->m_type==ASSET_TYPE_CLIPMAP_PVS) && !asset->IsReference()) map=static_cast<const clipMap_t*>(asset->m_ptr);
    std::cerr << "SUBMODELS " << (map?map->numSubModels:0) << std::endl;
    require(map && map->numSubModels>20 && map->numSubModels<=10000 && map->cmodels);
    std::ofstream out("map-triggers.json");out<<std::setprecision(9)<<"[";
    for(unsigned i=1;i<map->numSubModels;++i) {
        const auto& model=map->cmodels[i];const auto& info=model.info?*model.info:map->info;
        require(info.leafbrushNodes && info.brushes && info.leafbrushNodesCount<=1000000);
        std::vector<unsigned> stack{static_cast<unsigned>(model.leaf.leafBrushNode)};
        std::set<unsigned> nodes,brushes;
        while(!stack.empty()) {
            auto index=stack.back();stack.pop_back();require(index<info.leafbrushNodesCount);if(!nodes.insert(index).second)continue;
            const auto& n=info.leafbrushNodes[index];
            if(!n.contents) continue;
            if(n.leafBrushCount>0) {require(n.data.leaf.brushes);for(int j=0;j<n.leafBrushCount;++j) {auto b=n.data.leaf.brushes[j];require(b<info.numBrushes);brushes.insert(b);}}
            else {if(n.leafBrushCount<0)stack.push_back(index+1);for(unsigned j=0;j<2;++j) {require(n.data.children.childOffset[j]>0);stack.push_back(index+n.data.children.childOffset[j]);}}
        }
        if(i>1)out<<',';out<<"{\"model\":"<<i<<",\"min\":";vector(out,model.mins);out<<",\"max\":";vector(out,model.maxs);out<<",\"hulls\":[";
        bool first=true;
        for(auto index:brushes) {
            const auto& b=info.brushes[index];require(b.numsides<=10000 && (!b.numsides || b.sides));
            if(!first)out<<',';first=false;out<<"{\"brush\":"<<index<<",\"min\":";vector(out,b.mins);out<<",\"max\":";vector(out,b.maxs);out<<",\"planes\":[";
            for(unsigned j=0;j<b.numsides;++j) {const auto* plane=b.sides[j].plane;require(plane && std::isfinite(plane->dist));if(j)out<<',';out<<"{\"normal\":";vector(out,plane->normal);out<<",\"distance\":"<<plane->dist<<'}';}
            out<<"]}";
        }
        out<<"]}";
    }
    out<<']';require(out.good());
}
