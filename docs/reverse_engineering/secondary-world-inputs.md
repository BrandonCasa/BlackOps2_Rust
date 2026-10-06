# Secondary world corner inputs

The source's 489,788-byte secondary stream and 449,721 uint16 local corner indexes
are exported externally and SHA256-bound to the accepted geometry snapshot.
`secondary-corners.json` accounts for every one of 6,469 surfaces, retaining source
format, stream byte offset and exact snapshot corner span. Local indexes select
`byte_offset + local_index * qualified_stride`; sharing a geometry position does
not merge corners or erase attribute seams.

## Qualified input layouts

These are original D3D input-layout observations, before shader interpretation.
Float16 channels decode to f32; byte colors decode to UNORM channels. No shader
unpacking, coordinate change or attribute default is applied.

| Source format | Stride | Secondary inputs | Surfaces | Corners |
| --- | ---: | --- | ---: | ---: |
| 0 | 0 | none | 4,154 | 281,460 |
| 1 | 4 | TEXCOORD3 half2 at0 | 313 | 6,972 |
| 2 | 8 | TEXCOORD3 half2 at0; TEXCOORD5 UNORM8x4 at4 | 1,679 | 147,819 |
| 4 | 12 | TEXCOORD3 half4 at0; TEXCOORD5 UNORM8x4 at8 | 105 | 3,186 |
| 5 | 16 | TEXCOORD3 half4 at0; TEXCOORD5 UNORM8x4 at8; TEXCOORD6 UNORM8x4 at12 | 209 | 10,173 |
| 7 | 16 | TEXCOORD3 half4 at0; TEXCOORD4 half2 at8; TEXCOORD5 UNORM8x4 at12 | 1 | 9 |
| 3 | 8 | TEXCOORD3 half4 at0 | 8 | 102 |

Formats0/1/2/4 come from frame6325 layout correlations; format5 from actual
frame40264 event4692; format7 from frame119013 event7354, correlated to source
surface3844. Formats5 and7 have equal strides but different field layouts.
Format3 is qualified by the later actual lit frame27970 event4574/source386;
its secondary stream has half4 at0 and stride8. The earlier shadow observation
had no secondary binding.

## Admission and validation

`assets::secondary_stream::load_secondary_corners` accepts the sibling manifest,
an already admitted GeometrySnapshot and its admitted SHA256 identity. It uses
the existing safe relative-file dependency boundary and bounded SHA256 admission.
Manifest cap16MiB, stream64MiB and local indexes2MiB bound allocation. Surface
format/span mismatches, unsupported layouts, nonfinite half channels, arithmetic
overflow, truncation and identities are explicit errors. The standalone sibling
API preserves existing appearance v1 and diagnostic v2 loading.

The native external producer now emits local indexes alongside the accepted
main records; `export_secondary.py` emits the manifest and complete accounting.
The full export has zero qualified record bounds failures. Rust opt-in validation
decodes all 449,721 corners across all 6,469 surfaces and seven qualified formats.
Strict full loading succeeds; unknown format categories remain Unsupported. Synthetic tests verify seams,
distinct layouts/colors, missing/truncated/unsupported data, finite channels,
offset overflow and snapshot/dependency identities.

No original shader appearance or renderer routing integration follows from this
input admission. A rendering pass must request its evidenced input semantics and
reject missing required attributes. Fidelity remains INVESTIGATING.
