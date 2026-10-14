#[derive(Clone, Copy, Default)]
pub(in crate::material) enum RuntimeFeature {
    #[default]
    None,
    Glass,
    VertexDiffuse,
    VertexDiffuseSpecular,
    MaskedColorMatrix,
    TranslucentColorMatrix,
    ColorMatrix,
    AttachmentCamo,
    Scrolling,
    IndirectDiffuseSpecular,
    IndirectDiffuse,
    MaskedDetail,
}

#[derive(Clone, Copy, Default)]
pub(in crate::material) struct OrdinaryRecipe {
    pub(in crate::material) reveal: bool,
    pub(in crate::material) flag: bool,
    pub(in crate::material) vertex_lighting: bool,
    pub(in crate::material) lightmap_routes: bool,
    pub(in crate::material) color_matrix: bool,
    pub(in crate::material) runtime: RuntimeFeature,
    pub(in crate::material) expected: &'static str,
    pub(in crate::material) normal: bool,
    pub(in crate::material) specular: u8,
    pub(in crate::material) coverage: u8,
    pub(in crate::material) detail: bool,
    pub(in crate::material) occlusion: u8,
    pub(in crate::material) camo: bool,
    pub(in crate::material) programs: Option<[&'static str; 3]>,
}

pub(in crate::material) fn ordinary_recipe(family: &str) -> Option<OrdinaryRecipe> {
    Some(match family {
        "wpc_lit_sm_b0c0n0s0v0_efz5w56f" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_716f0714.hlsl",
            normal: true,
            specular: 1,
            coverage: 1,
            reveal: true,
            programs: Some([
                "pimp_shader_lmap_b89a2252.hlsl",
                "pimp_shader_lmap_3324e8aa.hlsl",
                "pimp_shader_lmap_3400e22.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_f1c6978a.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            detail: true,
            flag: true,
            programs: Some([
                "pimp_shader_lprobe_3f4a954b.hlsl",
                "pimp_shader_lprobe_66a8e47e.hlsl",
                "pimp_shader_lprobe_a824e6bf.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_flag_b0c0s0_6406ez95" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_b781f149.hlsl",
            specular: 1,
            coverage: 1,
            flag: true,
            programs: Some([
                "pimp_shader_lprobe_37fa2fe.hlsl",
                "pimp_shader_lprobe_5a9dd3cb.hlsl",
                "pimp_shader_lprobe_e55840c0.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0d0n0_w65788j0" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_bb12e125.hlsl",
            normal: true,
            detail: true,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_e2f09010.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0n0x0_8f47740f" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            runtime: RuntimeFeature::VertexDiffuseSpecular,
            expected: "pimp_shader_lprobe_181d1c0b.hlsl",
            normal: true,
            specular: 2,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_41ff6d3e.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_b0c0_50077370" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_be61ff8.hlsl",
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_bcc4a7bc.hlsl",
                "pimp_shader_lprobe_8a9b1683.hlsl",
                "pimp_shader_lprobe_52046ecd.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_b0c0n0_322zqw73" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_37d3284d.hlsl",
            normal: true,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_6e315978.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_b0c0s0_41731e52" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_e044b950.hlsl",
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_bcc4a7bc.hlsl",
                "pimp_shader_lprobe_8a9b1683.hlsl",
                "pimp_shader_lprobe_b9a6c865.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0_475q06w3" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            runtime: RuntimeFeature::VertexDiffuse,
            expected: "pimp_shader_lprobe_7d8789b8.hlsl",
            programs: Some([
                "pimp_shader_lprobe_bcc4a7bc.hlsl",
                "pimp_shader_lprobe_8a9b1683.hlsl",
                "pimp_shader_lprobe_2465f88d.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0n0x0_619829wj" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_ae06fc7b.hlsl",
            normal: true,
            specular: 2,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_f7e48d4e.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0_j60qqj0e" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_cb9c69c8.hlsl",
            programs: Some([
                "pimp_shader_lprobe_bcc4a7bc.hlsl",
                "pimp_shader_lprobe_8a9b1683.hlsl",
                "pimp_shader_lprobe_927e18fd.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0n0s0_wqqj839w" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_1c0bf8d5.hlsl",
            normal: true,
            specular: 1,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_45e989e0.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_r0c0n0_3qf192ew" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_f7a95e7d.hlsl",
            normal: true,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_ae4b2f48.hlsl",
            ]),
            ..Default::default()
        },
        "mlv_lit_sm_t0c0n0s0_9jfj95q5" => OrdinaryRecipe {
            vertex_lighting: true,
            lightmap_routes: true,
            expected: "pimp_shader_lprobe_e183a077.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            programs: Some([
                "pimp_shader_lprobe_63b41dee.hlsl",
                "pimp_shader_lprobe_55ebacd1.hlsl",
                "pimp_shader_lprobe_b861d142.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_t0c0n0s0_f933q84u" => OrdinaryRecipe {
            color_matrix: true,
            runtime: RuntimeFeature::MaskedColorMatrix,
            expected: "pimp_shader_lprobe_c793a237.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_9e71d302.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0s0_68f25z0u" => OrdinaryRecipe {
            color_matrix: true,
            runtime: RuntimeFeature::TranslucentColorMatrix,
            expected: "pimp_shader_lprobe_fa618ca5.hlsl",
            normal: true,
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_a383fd90.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0s0_q94f27qw" => OrdinaryRecipe {
            color_matrix: true,
            runtime: RuntimeFeature::TranslucentColorMatrix,
            expected: "pimp_shader_lprobe_c654bb10.hlsl",
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_9fb6ca25.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0v0_3559e8jj" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_66c64d74.hlsl",
            normal: true,
            coverage: 1,
            reveal: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_3f243c41.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_b0c0n0v0_ew347fe3" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_82e99c4e.hlsl",
            normal: true,
            coverage: 1,
            reveal: true,
            programs: Some([
                "pimp_shader_lmap_d107eac0.hlsl",
                "pimp_shader_lmap_5ab92038.hlsl",
                "pimp_shader_lmap_f0c69578.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0d0n0_33ffej1u" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_ecd7a93c.hlsl",
            normal: true,
            detail: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_b535d809.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0d0n0s0_qwq4q449" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_7750f94.hlsl",
            normal: true,
            specular: 1,
            detail: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_5e977ea1.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_t0c0d0n0s0_68eww765" => OrdinaryRecipe {
            runtime: RuntimeFeature::MaskedDetail,
            expected: "pimp_shader_lprobe_fafd5736.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            detail: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_a31f2603.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0o0_3z86zq2z" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_3c17e0d4.hlsl",
            normal: true,
            specular: 1,
            occlusion: 1,
            camo: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_65f591e1.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0o0_909j993q" => OrdinaryRecipe {
            color_matrix: true,
            expected: "pimp_shader_lprobe_4dc2aa8d.hlsl",
            normal: true,
            specular: 1,
            occlusion: 1,
            camo: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_1420dbb8.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0s0o0_jf00fe4q" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_29115a54.hlsl",
            normal: true,
            specular: 1,
            coverage: 1,
            occlusion: 2,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_70f32b61.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0_ze50j7jz" => OrdinaryRecipe {
            color_matrix: true,
            runtime: RuntimeFeature::ColorMatrix,
            expected: "pimp_shader_lprobe_3a1bfa95.hlsl",
            normal: true,
            specular: 1,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_63f98ba0.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0_7j414q0z" => OrdinaryRecipe {
            runtime: RuntimeFeature::AttachmentCamo,
            expected: "pimp_shader_lprobe_9eb27c7c.hlsl",
            normal: true,
            specular: 1,
            camo: true,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_c7500d49.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0_4w3wqfe5" => OrdinaryRecipe {
            runtime: RuntimeFeature::Scrolling,
            expected: "pimp_shader_lprobe_4bceb0cc.hlsl",
            normal: true,
            specular: 1,
            programs: Some([
                "pimp_shader_lprobe_7bb0619c.hlsl",
                "pimp_shader_lprobe_4defd0a3.hlsl",
                "pimp_shader_lprobe_122cc1f9.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0x0_53144qq1" => OrdinaryRecipe {
            runtime: RuntimeFeature::IndirectDiffuseSpecular,
            expected: "pimp_shader_lprobe_4fd85412.hlsl",
            normal: true,
            specular: 2,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_163a2527.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_r0c0_5137qq99" => OrdinaryRecipe {
            runtime: RuntimeFeature::IndirectDiffuse,
            expected: "pimp_shader_lprobe_2a42c1a1.hlsl",
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_73a0b094.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0s0_21fz2e61" => OrdinaryRecipe {
            runtime: RuntimeFeature::Glass,
            expected: "pimp_shader_lprobe_6741885c.hlsl",
            normal: true,
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_3ea3f969.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0s0_91j5z9q9" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_8bb4c6fc.hlsl",
            normal: true,
            specular: 1,
            coverage: 1,
            ..Default::default()
        },
        "wpc_lit_sm_r0c0n0s0_1zzj1138" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_353f0d80.hlsl",
            normal: true,
            specular: 1,
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0s0_zqq1fze7" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_4bceb0cc.hlsl",
            normal: true,
            specular: 1,
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0x0_q361191u" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_f9c3b462.hlsl",
            normal: true,
            specular: 2,
            ..Default::default()
        },
        "wpc_lit_sm_r0c0n0x0_f488034e" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_9a71a1e7.hlsl",
            normal: true,
            specular: 2,
            ..Default::default()
        },
        "mc_lit_sm_r0c0_77e21qq8" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_9c5921d1.hlsl",
            ..Default::default()
        },
        "mc_lit_sm_r0c0n0_2z223015" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_a06c1664.hlsl",
            normal: true,
            ..Default::default()
        },
        "mc_lit_sm_t0c0n0s0_47jq982j" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_b646e86e.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            ..Default::default()
        },
        "mc_lit_sm_r0c0s0_986ezzjq" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_77fb8779.hlsl",
            specular: 1,
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_2e19f64c.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_r0c0n0_80fe30wz" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_c6b996da.hlsl",
            normal: true,
            programs: Some([
                "pimp_shader_lmap_d107eac0.hlsl",
                "pimp_shader_lmap_5ab92038.hlsl",
                "pimp_shader_lmap_b4969fec.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_t0c0_90wz6fe2" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_61d17973.hlsl",
            coverage: 2,
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_38330846.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_t0c0_2w3887e6" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_2d3e6f96.hlsl",
            coverage: 2,
            programs: Some([
                "pimp_shader_lmap_bfa6d365.hlsl",
                "pimp_shader_lmap_3418199d.hlsl",
                "pimp_shader_lmap_5f1166a0.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_t0c0n0_9ew0jz24" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_439f5633.hlsl",
            normal: true,
            coverage: 2,
            programs: Some([
                "pimp_shader_lmap_d107eac0.hlsl",
                "pimp_shader_lmap_5ab92038.hlsl",
                "pimp_shader_lmap_31b05f05.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_t0c0n0_9qf6e4qj" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_5de44ec6.hlsl",
            normal: true,
            coverage: 2,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_4063ff3.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_t0c0n0s0_weef63f3" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_b019cd69.hlsl",
            normal: true,
            specular: 1,
            coverage: 2,
            programs: Some([
                "pimp_shader_lmap_b89a2252.hlsl",
                "pimp_shader_lmap_3324e8aa.hlsl",
                "pimp_shader_lmap_c236c45f.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0_fw0jf955" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_5c2357e1.hlsl",
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_5c126d4.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0s0_805w75j2" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_b781f149.hlsl",
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_eb01efa5.hlsl",
                "pimp_shader_lprobe_dd5e5e9a.hlsl",
                "pimp_shader_lprobe_ee63807c.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_b0c0n0_19f3588f" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_4f8f060e.hlsl",
            normal: true,
            coverage: 1,
            programs: Some([
                "pimp_shader_lmap_d107eac0.hlsl",
                "pimp_shader_lmap_5ab92038.hlsl",
                "pimp_shader_lmap_3da00f38.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_b0c0_6j30z67q" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_212e3fab.hlsl",
            coverage: 1,
            programs: Some([
                "pimp_shader_lmap_bfa6d365.hlsl",
                "pimp_shader_lmap_3418199d.hlsl",
                "pimp_shader_lmap_5301369d.hlsl",
            ]),
            ..Default::default()
        },
        "mc_lit_sm_b0c0n0_67fqj7jf" => OrdinaryRecipe {
            expected: "pimp_shader_lprobe_60166054.hlsl",
            normal: true,
            coverage: 1,
            programs: Some([
                "pimp_shader_lprobe_347155f7.hlsl",
                "pimp_shader_lprobe_22ee4c8.hlsl",
                "pimp_shader_lprobe_39f41161.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_r0c0s0_qfqz7850" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_5b9e3425.hlsl",
            specular: 1,
            programs: Some([
                "pimp_shader_lmap_d63b1bf7.hlsl",
                "pimp_shader_lmap_5d85d10f.hlsl",
                "pimp_shader_lmap_29b13d13.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_r0c0x0_5j63fj97" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_f4d09842.hlsl",
            specular: 2,
            programs: Some([
                "pimp_shader_lmap_d63b1bf7.hlsl",
                "pimp_shader_lmap_5d85d10f.hlsl",
                "pimp_shader_lmap_86ff9174.hlsl",
            ]),
            ..Default::default()
        },
        "wpc_lit_sm_b0c0s0_3f3q946z" => OrdinaryRecipe {
            lightmap_routes: true,
            expected: "pimp_shader_lmap_d2a8a4f1.hlsl",
            specular: 1,
            coverage: 1,
            programs: Some([
                "pimp_shader_lmap_d63b1bf7.hlsl",
                "pimp_shader_lmap_5d85d10f.hlsl",
                "pimp_shader_lmap_a087adc7.hlsl",
            ]),
            ..Default::default()
        },
        _ => return None,
    })
}
