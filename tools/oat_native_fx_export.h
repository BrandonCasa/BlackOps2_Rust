#pragma once
#include "Dumping/AbstractAssetDumper.h"
#include "Game/T6/T6.h"
#include <nlohmann/json.hpp>
namespace native_fx_export {
using namespace T6;
using J=nlohmann::json;
inline J range(FxFloatRange r){return J::array({r.base,r.amplitude});}
inline J range(FxIntRange r){return J::array({r.base,r.amplitude});}
inline J vec(const float* v,int n=3){J a=J::array();for(int i=0;i<n;i++)a.push_back(v[i]);return a;}
inline J ranges(const FxFloatRange* v){J a=J::array();for(int i=0;i<3;i++)a.push_back(range(v[i]));return a;}
inline J velocity(const FxElemVelStateInFrame& v){return J{{"velocity",J::array({vec(v.velocity.base.v),vec(v.velocity.amplitude.v)})},{"delta",J::array({vec(v.totalDelta.base.v),vec(v.totalDelta.amplitude.v)})}};}
inline J state(const FxElemVisualState& v){J c=J::array();for(int i=0;i<4;i++)c.push_back(static_cast<unsigned char>(v.color[i]));return J{{"color",c},{"rotation_delta",v.rotationDelta},{"rotation_total",v.rotationTotal},{"size",vec(v.size,2)},{"scale",v.scale}};}
inline std::string ref(FxEffectDefRef v){return v.name?v.name:"";}
class Dumper final:public AbstractAssetDumper<T6::AssetFx>{
void DumpAsset(AssetDumpingContext& context,const XAssetInfo<T6::AssetFx::Type>& asset)override{
 const auto& f=*asset.Asset();const int total=f.elemDefCountLooping+f.elemDefCountOneShot+f.elemDefCountEmission;if(total<0 || total>4096)return;
 J d{{"name",f.name},{"flags",f.flags},{"looping_count",f.elemDefCountLooping},{"oneshot_count",f.elemDefCountOneShot},{"emission_count",f.elemDefCountEmission},{"loop_life",f.msecLoopingLife},{"oneshot_life",f.msecNonLoopingLife},{"elements",J::array()}};
 for(int i=0;i<total;i++){
  const auto& e=f.elemDefs[i];J visuals=J::array();
  for(int v=0;v<static_cast<unsigned char>(e.visualCount);v++){
   if(e.elemType==FX_ELEM_TYPE_DECAL){if(e.visuals.markArray)for(int m=0;m<2;m++){const auto* material=e.visuals.markArray[v].materials[m];if(material)visuals.push_back(material->info.name);}continue;}
   const auto& visual=e.visualCount>1?e.visuals.array[v]:e.visuals.instance;const char* name=nullptr;
   if(e.elemType<=FX_ELEM_TYPE_CLOUD && visual.material)name=visual.material->info.name;
   else if(e.elemType==FX_ELEM_TYPE_MODEL && visual.model)name=visual.model->name;
   else if(e.elemType==FX_ELEM_TYPE_RUNNER)name=visual.effectDef.name;
   else if(e.elemType==FX_ELEM_TYPE_SOUND)name=visual.soundName;
   if(name)visuals.push_back(name);
  }
  J vel=J::array(),vis=J::array();
  if(e.velSamples)for(int n=0;n<=static_cast<unsigned char>(e.velIntervalCount);n++)vel.push_back(J{{"local",velocity(e.velSamples[n].local)},{"world",velocity(e.velSamples[n].world)}});
  if(e.visSamples)for(int n=0;n<=static_cast<unsigned char>(e.visStateIntervalCount);n++)vis.push_back(J{{"base",state(e.visSamples[n].base)},{"amplitude",state(e.visSamples[n].amplitude)}});
  J elem{{"type",static_cast<int>(e.elemType)},{"flags",e.flags},{"looping",i<f.elemDefCountLooping},{"spawn",J::array({e.spawn.looping.intervalMsec,e.spawn.looping.count})},{"delay",range(e.spawnDelayMsec)},{"life",range(e.lifeSpanMsec)},{"origin",ranges(e.spawnOrigin)},{"radius",range(e.spawnOffsetRadius)},{"height",range(e.spawnOffsetHeight)},{"angles",ranges(e.spawnAngles)},{"angular_velocity",ranges(e.angularVelocity)},{"rotation",range(e.initialRotation)},{"gravity",range(e.gravity)},{"atlas",J::array({static_cast<int>(e.atlas.behavior),static_cast<int>(e.atlas.index),static_cast<int>(e.atlas.fps),static_cast<int>(e.atlas.loopCount),static_cast<int>(e.atlas.colIndexBits),static_cast<int>(e.atlas.rowIndexBits),e.atlas.entryCountAndIndexRange})},{"velocities",vel},{"states",vis},{"visuals",visuals},{"impact",ref(e.effectOnImpact)},{"death",ref(e.effectOnDeath)},{"emitted",ref(e.effectEmitted)},{"attached",ref(e.effectAttached)},{"sound",e.spawnSound.spawnSound?e.spawnSound.spawnSound:""},{"pivot",vec(e.billboardPivot.v,2)}};
  d["elements"].push_back(elem);
 }
 const auto file=context.OpenAssetFile("native_fx/"+asset.m_name+".json");if(file)*file<<d.dump();
}
};
}
