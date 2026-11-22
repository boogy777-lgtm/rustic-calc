@{
    # Only exact locked package versions with no usable legal text in the cached
    # crate archive appear here. Prefer the crate's exact .cargo_vcs_info
    # commit; when that tree has no license file, identify the release's grant
    # and pin the corresponding attributed text rather than inventing notices.
    LegalTexts = @{
        AccessKitApacheA55 = @{
            Path = 'third_party\license-notice-overrides\accesskit\a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/AccessKit/accesskit/a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'AccessKit 0.24.1 cached .cargo_vcs_info.json revision.'
            Sha256 = '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a'
        }
        AccessKitMitA55 = @{
            Path = 'third_party\license-notice-overrides\accesskit\a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/AccessKit/accesskit/a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'AccessKit 0.24.1 cached .cargo_vcs_info.json revision.'
            Sha256 = '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3'
        }
        AccessKitApacheC886 = @{
            Path = 'third_party\license-notice-overrides\accesskit\c88605b96d04431f9c3c792464a0f2f253480e94\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/AccessKit/accesskit/c88605b96d04431f9c3c792464a0f2f253480e94/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'AccessKit adapter/consumer cached .cargo_vcs_info.json revision.'
            Sha256 = '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a'
        }
        AccessKitMitC886 = @{
            Path = 'third_party\license-notice-overrides\accesskit\c88605b96d04431f9c3c792464a0f2f253480e94\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/AccessKit/accesskit/c88605b96d04431f9c3c792464a0f2f253480e94/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'AccessKit adapter/consumer cached .cargo_vcs_info.json revision.'
            Sha256 = '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3'
        }
        ClipboardWinBsl = @{
            Path = 'third_party\license-notice-overrides\clipboard-win\ed27008ae5d1f10cfbb266de039f0fb35fd05a56\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/DoumanAsh/clipboard-win/ed27008ae5d1f10cfbb266de039f0fb35fd05a56/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'clipboard-win 5.4.3 cached .cargo_vcs_info.json revision.'
            Sha256 = 'c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566'
        }
        GlRsApacheEa503 = @{
            Path = 'third_party\license-notice-overrides\gl-rs\ea503e8d5fb6d73c6030e6191ce738cd3bf3433e\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/brendanzab/gl-rs/ea503e8d5fb6d73c6030e6191ce738cd3bf3433e/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'gl_generator 0.14.0 cached .cargo_vcs_info.json revision.'
            Sha256 = 'cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30'
        }
        GlRsApacheF150 = @{
            Path = 'third_party\license-notice-overrides\gl-rs\f150967b1c44ae888e6676f93f639ebc82771bdc\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/brendanzab/gl-rs/f150967b1c44ae888e6676f93f639ebc82771bdc/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'khronos_api 3.1.0 cached .cargo_vcs_info.json revision.'
            Sha256 = 'cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30'
        }
        LyonApacheDb98 = @{
            Path = 'third_party\license-notice-overrides\lyon\db98d26a1a90076a7a894262dcb51117eef643c8\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/nical/lyon/db98d26a1a90076a7a894262dcb51117eef643c8/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'lyon algorithms, geometry, and tessellation cached .cargo_vcs_info.json revision.'
            Sha256 = '216486f29671a4262efe32af6d84a75bef398127f8c5f369b5c8305983887a06'
        }
        LyonMitDb98 = @{
            Path = 'third_party\license-notice-overrides\lyon\db98d26a1a90076a7a894262dcb51117eef643c8\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/nical/lyon/db98d26a1a90076a7a894262dcb51117eef643c8/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'lyon algorithms, geometry, and tessellation cached .cargo_vcs_info.json revision.'
            Sha256 = '80f275e90d799911ed3830a7f242a2ef5a4ade2092fe0aa07bfb2d2cf2f2b95e'
        }
        LyonApache562 = @{
            Path = 'third_party\license-notice-overrides\lyon\56219c95c658d4f39de7b190d98e57d5773ba241\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/nical/lyon/56219c95c658d4f39de7b190d98e57d5773ba241/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'lyon path, extra, and top-level crate cached .cargo_vcs_info.json revision.'
            Sha256 = '216486f29671a4262efe32af6d84a75bef398127f8c5f369b5c8305983887a06'
        }
        LyonMit562 = @{
            Path = 'third_party\license-notice-overrides\lyon\56219c95c658d4f39de7b190d98e57d5773ba241\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/nical/lyon/56219c95c658d4f39de7b190d98e57d5773ba241/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'lyon path, extra, and top-level crate cached .cargo_vcs_info.json revision.'
            Sha256 = '80f275e90d799911ed3830a7f242a2ef5a4ade2092fe0aa07bfb2d2cf2f2b95e'
        }
        RspolibMit = @{
            Path = 'third_party\license-notice-overrides\rspolib\8170d99ebad29d894712ccb3dc6d0be2198d3055\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/mondeja/rspolib/8170d99ebad29d894712ccb3dc6d0be2198d3055/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'rspolib 0.1.2 cached .cargo_vcs_info.json revision; crate package archive omits the repository-root file.'
            Sha256 = '4fe9b5fce8fd028261959d039d19ef66182be088ab48ffa97dbeaef7b35e39c3'
        }
        DtorApacheE8c8 = @{
            Path = 'third_party\license-notice-overrides\rust-ctor\e8c8b4a82de72311a5257db0502ced728081e745\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/mmastrac/rust-ctor/e8c8b4a82de72311a5257db0502ced728081e745/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'dtor 0.0.6 cached .cargo_vcs_info.json revision.'
            Sha256 = 'a8ad31b1c3f40dca5a84119351b8fa8ddc868edd77fad8a8ebf6d8f2d16fa4ae'
        }
        DtorMitE8c8 = @{
            Path = 'third_party\license-notice-overrides\rust-ctor\e8c8b4a82de72311a5257db0502ced728081e745\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/mmastrac/rust-ctor/e8c8b4a82de72311a5257db0502ced728081e745/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'dtor 0.0.6 cached .cargo_vcs_info.json revision.'
            Sha256 = 'bccaa8b6c09f94e81f06696e179dbe058464bbdfbc823b6d49cada1d71e84ac3'
        }
        DtorApacheB051 = @{
            Path = 'third_party\license-notice-overrides\rust-ctor\b05136ac964049a1ee29de763d0b119c42258567\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/mmastrac/rust-ctor/b05136ac964049a1ee29de763d0b119c42258567/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'dtor-proc-macro 0.0.5 cached .cargo_vcs_info.json revision.'
            Sha256 = 'a8ad31b1c3f40dca5a84119351b8fa8ddc868edd77fad8a8ebf6d8f2d16fa4ae'
        }
        DtorMitB051 = @{
            Path = 'third_party\license-notice-overrides\rust-ctor\b05136ac964049a1ee29de763d0b119c42258567\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/mmastrac/rust-ctor/b05136ac964049a1ee29de763d0b119c42258567/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'dtor-proc-macro 0.0.5 cached .cargo_vcs_info.json revision.'
            Sha256 = 'bccaa8b6c09f94e81f06696e179dbe058464bbdfbc823b6d49cada1d71e84ac3'
        }
        ProfilingApache = @{
            Path = 'third_party\license-notice-overrides\profiling\8271551172eb6fa4cba47369aedd93790c623df9\LICENSE-APACHE'
            Label = 'LICENSE-APACHE'
            SourceUrl = 'https://raw.githubusercontent.com/aclysma/profiling/8271551172eb6fa4cba47369aedd93790c623df9/LICENSE-APACHE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'profiling 1.0.18 and profiling-procmacros 1.0.18 cached .cargo_vcs_info.json revision.'
            Sha256 = '10d30a673cd5e9349bdc02aeb48f14b3386d27d0da32df8f0a555d4aa16aa551'
        }
        ProfilingMit = @{
            Path = 'third_party\license-notice-overrides\profiling\8271551172eb6fa4cba47369aedd93790c623df9\LICENSE-MIT'
            Label = 'LICENSE-MIT'
            SourceUrl = 'https://raw.githubusercontent.com/aclysma/profiling/8271551172eb6fa4cba47369aedd93790c623df9/LICENSE-MIT'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'profiling 1.0.18 and profiling-procmacros 1.0.18 cached .cargo_vcs_info.json revision.'
            Sha256 = 'c8167fdeeed46d3f244d3f85c5bf998ce889343691c32be2c61a8bc4b5c08333'
        }
        StackSafeApache = @{
            Path = 'third_party\license-notice-overrides\stacksafe\a5c029b7d3a5fe6771ecd79266b6e21313cfa174\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/fast/stacksafe/a5c029b7d3a5fe6771ecd79266b6e21313cfa174/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'stacksafe 0.1.4 and stacksafe-macro 0.1.4 cached .cargo_vcs_info.json revision.'
            Sha256 = 'c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4'
        }
        RustDebugMit = @{
            Path = 'third_party\license-notice-overrides\rust_debug\93f414cd572b5c2781df93dbdc068ed13443d05a\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/nical/rust_debug/93f414cd572b5c2781df93dbdc068ed13443d05a/LICENSE'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'svg_fmt 0.4.5 cached .cargo_vcs_info.json revision; its MIT/Apache-2.0 expression permits use of the exact upstream MIT alternative.'
            Sha256 = '7e98bed7f100747aaae7d82ab94fbe6b76fd6b84b09a3aaa413bff8196d0fbb1'
        }
        ZuneLicenseNotice = @{
            Path = 'third_party\license-notice-overrides\zune-image\69502ce83fdfecdd0beefd677e2abb3781b29d98\LICENSE.md'
            Label = 'LICENSE.md'
            SourceUrl = 'https://raw.githubusercontent.com/etemesi254/zune-image/69502ce83fdfecdd0beefd677e2abb3781b29d98/LICENSE.md'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'zune-inflate 0.2.54 cached .cargo_vcs_info.json revision; the crate archive omits repository-root license texts.'
            Sha256 = 'e870d7fd7819a9f0453c62798cb52ae95d148a8c91dc2de1d983f997143c72de'
        }
        ZuneZlib = @{
            Path = 'third_party\license-notice-overrides\zune-image\69502ce83fdfecdd0beefd677e2abb3781b29d98\LICENSE-ZLIB'
            Label = 'LICENSE-ZLIB'
            SourceUrl = 'https://raw.githubusercontent.com/etemesi254/zune-image/69502ce83fdfecdd0beefd677e2abb3781b29d98/LICENSE-ZLIB'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'zune-inflate 0.2.54 cached .cargo_vcs_info.json revision.'
            Sha256 = '7fa429541e55b1509909e058f2d21a37467e4958ec713b357f6e0cf9dc4ee352'
        }
        TaffyMit010 = @{
            Path = 'third_party\license-notice-overrides\taffy\639f0acdd20f914f847abffbeaa903d66cae4882\LICENSE.md'
            Label = 'LICENSE.md'
            SourceUrl = 'https://raw.githubusercontent.com/DioxusLabs/taffy/639f0acdd20f914f847abffbeaa903d66cae4882/LICENSE.md'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'taffy 0.10.1 cached .cargo_vcs_info.json revision.'
            Sha256 = 'a7083ef3dd41a65eeb2162c2c091b0082e2724fb4c6ed029b363c7840d244ecd'
        }
        TaffyMit09 = @{
            Path = 'third_party\license-notice-overrides\taffy\639f0acdd20f914f847abffbeaa903d66cae4882\LICENSE.md'
            Label = 'LICENSE.md'
            SourceUrl = 'https://raw.githubusercontent.com/DioxusLabs/taffy/599b1b7d7410d97e13bafc4c01fbe6d7a9cf060a/LICENSE.md'
            SourceKind = 'exact upstream commit'
            ProvenanceNote = 'taffy 0.9.0 cached .cargo_vcs_info.json revision; identical to the 0.10.1 license file.'
            Sha256 = 'a7083ef3dd41a65eeb2162c2c091b0082e2724fb4c6ed029b363c7840d244ecd'
        }
        ConvertCaseMit = @{
            Path = 'third_party\license-notice-overrides\convert-case\f72ca63c9d579fbab22e361c76e39d31d1e86a2e\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/rutrum/convert-case/f72ca63c9d579fbab22e361c76e39d31d1e86a2e/LICENSE'
            SourceKind = 'later upstream publisher license'
            ProvenanceNote = '0.4.0 Cargo metadata already declares MIT and names David Purdum; the pinned upstream commit adds the authored MIT text.'
            Sha256 = '797087750c4103075e96bb7a60202a040812f8679ae2f7263148a1cc0b298d28'
        }
        SimdHelpersMit = @{
            Path = 'third_party\license-notice-overrides\simd_helpers\82040194cd05affb060bf94d6f19f82a771d07fb\LICENSE'
            Label = 'LICENSE'
            SourceUrl = 'https://raw.githubusercontent.com/lu-zero/simd_helpers/82040194cd05affb060bf94d6f19f82a771d07fb/LICENSE'
            SourceKind = 'later upstream publisher license'
            ProvenanceNote = '0.1.0 Cargo metadata already declares MIT and names Luca Barbato; the pinned upstream commit adds only the authored MIT text.'
            Sha256 = 'd69f24ad84ec2ade64c0b68bdb31b41170e997b158370342056918329cc9af1e'
        }
        SeahashMit = @{
            Path = 'third_party\license-notice-overrides\seahash\3f0633239f9beb10b9428589c54f876697a08599\copyright'
            Label = 'debian/copyright'
            SourceUrl = 'https://raw.githubusercontent.com/deepin-community/rust-seahash/3f0633239f9beb10b9428589c54f876697a08599/debian/copyright'
            SourceKind = 'distribution attribution for the exact release'
            ProvenanceNote = 'The pinned distro source packages SeaHash 4.1.0; its upstream attribution and MIT text match the crate declaration and authors. No LICENSE is present in the upstream release tree.'
            Sha256 = '268296361f3ec03e3dfb8a893c35318226119c818f5e8b7866d209e24beda60f'
        }
        MacAuthorNotice = @{
            Path = 'third_party\license-notice-overrides\rust-mac\66afc663b68a65633ea165c742b5a9c6734581c2\README.md'
            Label = 'README.md (author and license declaration)'
            SourceUrl = 'https://raw.githubusercontent.com/reem/rust-mac/66afc663b68a65633ea165c742b5a9c6734581c2/README.md'
            SourceKind = 'exact upstream release author and license declaration'
            ProvenanceNote = 'mac 0.1.1 tag, authored by Jonathan Reem; upstream offers MIT or Apache-2.0. Use the Apache-2.0 option.'
            Sha256 = '8edb7a912d295acf1e1cfeafdd19a6813a63b301c7bbfda3c67f65509bd8d8cc'
        }
        MacApacheText = @{
            Path = 'third_party\license-notice-overrides\gl-rs\ea503e8d5fb6d73c6030e6191ce738cd3bf3433e\LICENSE'
            Label = 'Apache License 2.0 (the declared option)'
            SourceUrl = 'https://www.apache.org/licenses/LICENSE-2.0.txt'
            SourceKind = 'official full text of the license declared at the exact release'
            ProvenanceNote = 'The existing verbatim Apache text is byte-identical to the Apache Software Foundation text; the separately preserved mac 0.1.1 README supplies the grant and author attribution.'
            Sha256 = 'cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30'
        }
    }
    Packages = @{
        'accesskit|0.24.1' = @{ Files = @('AccessKitApacheA55', 'AccessKitMitA55') }
        'accesskit_windows|0.34.0' = @{ Files = @('AccessKitApacheC886', 'AccessKitMitC886') }
        'accesskit_consumer|0.38.0' = @{ Files = @('AccessKitApacheC886', 'AccessKitMitC886') }
        'accesskit_winit|0.33.2' = @{ Files = @('AccessKitApacheC886') }
        'clipboard-win|5.4.3' = @{ Files = @('ClipboardWinBsl') }
        'gl_generator|0.14.0' = @{ Files = @('GlRsApacheEa503') }
        'khronos_api|3.1.0' = @{ Files = @('GlRsApacheF150') }
        'lyon_algorithms|1.0.22' = @{ Files = @('LyonApacheDb98', 'LyonMitDb98') }
        'lyon_geom|1.0.20' = @{ Files = @('LyonApacheDb98', 'LyonMitDb98') }
        'lyon_tessellation|1.0.23' = @{ Files = @('LyonApacheDb98', 'LyonMitDb98') }
        'lyon_extra|1.1.0' = @{ Files = @('LyonApache562', 'LyonMit562') }
        'lyon_path|1.0.19' = @{ Files = @('LyonApache562', 'LyonMit562') }
        'lyon|1.0.19' = @{ Files = @('LyonApache562', 'LyonMit562') }
        'rspolib|0.1.2' = @{ Files = @('RspolibMit') }
        'dtor|0.0.6' = @{ Files = @('DtorApacheE8c8', 'DtorMitE8c8') }
        'dtor-proc-macro|0.0.5' = @{ Files = @('DtorApacheB051', 'DtorMitB051') }
        'profiling|1.0.18' = @{ Files = @('ProfilingApache', 'ProfilingMit') }
        'profiling-procmacros|1.0.18' = @{ Files = @('ProfilingApache', 'ProfilingMit') }
        'stacksafe|0.1.4' = @{ Files = @('StackSafeApache') }
        'stacksafe-macro|0.1.4' = @{ Files = @('StackSafeApache') }
        'svg_fmt|0.4.5' = @{ Files = @('RustDebugMit') }
        'zune-inflate|0.2.54' = @{ Files = @('ZuneLicenseNotice', 'ZuneZlib') }
        'taffy|0.10.1' = @{ Files = @('TaffyMit010') }
        'taffy|0.9.0' = @{ Files = @('TaffyMit09') }
        'convert_case|0.4.0' = @{ Files = @('ConvertCaseMit') }
        'simd_helpers|0.1.0' = @{ Files = @('SimdHelpersMit') }
        'seahash|4.1.0' = @{ Files = @('SeahashMit') }
        'mac|0.1.1' = @{ Files = @('MacAuthorNotice', 'MacApacheText') }
    }
}
