//! Le texte livré par les versions précédentes et reformulé depuis :
//! l'empreinte de chaque champ de fiche (dispositifs, préparations) tel
//! qu'il était livré avant la relecture de la 0.369.0.
//!
//! Ces fiches sont semées une fois puis appartiennent à l'équipe, sans
//! verrou par champ. Pour qu'une reformulation atteigne une base
//! existante sans toucher à ce que l'officine a écrit, un champ n'est
//! remplacé que si son contenu a **exactement** l'empreinte de l'ancien
//! texte livré : un champ réécrit par l'équipe n'a plus cette empreinte.
//!
//! L'empreinte est FNV-1a sur 64 bits, des octets UTF-8 du texte : pas de
//! dépendance, et une collision avec un texte réécrit par l'équipe est
//! improbable au point d'être négligeable.
//!
//! Pur ; les tables sont produites par comparaison des sources livrées
//! (voir `the_reworded_fingerprints_name_existing_fields`).

/// FNV-1a 64 bits.
pub fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// StarterDispositif: 152 champs, 55 fiches anciennes, 58 nouvelles
pub const DISPOSITIFS_REWORDED: &[(&str, &str, u64)] = &[
    ("Hydrocolloïde", "indication", 0x1c9083b2117ffd3f),
    ("Hydrocolloïde", "renewal", 0x0f69eeacd8953f15),
    ("Hydrocolloïde", "caution", 0x3d5f32e3585c24e4),
    ("Hydrocellulaire (mousse)", "indication", 0xdf7486e128dec004),
    ("Hydrocellulaire (mousse)", "lpp", 0xb9841c6f5ee5f284),
    ("Hydrocellulaire (mousse)", "caution", 0xf2955a57993856ae),
    ("Alginate de calcium", "indication", 0x8b000f6eb48ab7fa),
    ("Alginate de calcium", "renewal", 0x0bc85f24285221cb),
    ("Alginate de calcium", "caution", 0x108104d65f5c8ae7),
    ("Hydrofibre", "indication", 0xcc07f82d193c609c),
    ("Hydrofibre", "caution", 0x6d8e975daf7aaa77),
    ("Hydrogel", "indication", 0x4e5317a334c4951e),
    ("Hydrogel", "application", 0xcf24d7586f844a6f),
    ("Hydrogel", "caution", 0xfea743d40e8f2851),
    ("Interface et tulle gras", "renewal", 0x5dfe96fcc52f58d6),
    ("Interface et tulle gras", "lpp", 0x4f9ca8d73f8c5d3a),
    ("Interface et tulle gras", "caution", 0x194d48a57e9bea37),
    ("Pansement à l'argent", "application", 0xf479da833e09c6de),
    ("Pansement à l'argent", "renewal", 0x1edb9e7e933512b3),
    ("Pansement à l'argent", "lpp", 0xe860f485a41f4d4d),
    ("Pansement à l'argent", "caution", 0xbd04374e08c1e90e),
    (
        "Pansement au charbon actif",
        "indication",
        0x29a20d0452cb6fdc,
    ),
    ("Pansement au charbon actif", "renewal", 0x66f091cfbe010772),
    ("Pansement au charbon actif", "caution", 0x6b1d6cab64df77b6),
    (
        "Film adhésif semi-perméable",
        "indication",
        0xbc523dde97d876a7,
    ),
    (
        "Film adhésif semi-perméable",
        "application",
        0xfd9a22685292e622,
    ),
    ("Film adhésif semi-perméable", "caution", 0xeb663aa7a07f973b),
    (
        "Compresses stériles et non stériles",
        "application",
        0x4bd9aa7d3ea895a2,
    ),
    (
        "Compresses stériles et non stériles",
        "renewal",
        0xafbbcaa89e4e3510,
    ),
    (
        "Compresses stériles et non stériles",
        "lpp",
        0x835de501a9212e8f,
    ),
    (
        "Compresses stériles et non stériles",
        "caution",
        0x9790ccb135a7c9cf,
    ),
    (
        "Bande de maintien et bande de crêpe",
        "indication",
        0xc27b100ead128b99,
    ),
    (
        "Filet tubulaire de maintien",
        "indication",
        0x506fbecde0e62ea1,
    ),
    ("Filet tubulaire de maintien", "caution", 0x41f330b23a7f47f1),
    (
        "Sparadrap et adhésif hypoallergénique",
        "application",
        0xe50bd20d15accd8c,
    ),
    (
        "Sparadrap et adhésif hypoallergénique",
        "caution",
        0xd8b795f86c8ca7d2,
    ),
    (
        "Bas et chaussettes de compression",
        "indication",
        0x397d81c0625281e0,
    ),
    (
        "Bas et chaussettes de compression",
        "sizes",
        0xd534b28c5631491f,
    ),
    (
        "Bas et chaussettes de compression",
        "application",
        0x98f6dc5c9132b8b0,
    ),
    (
        "Bas et chaussettes de compression",
        "renewal",
        0xcfe44de4f454226d,
    ),
    (
        "Bas et chaussettes de compression",
        "lpp",
        0x6c2a52e8295d86e3,
    ),
    (
        "Bas et chaussettes de compression",
        "caution",
        0x40dde80b9786ba4a,
    ),
    (
        "Bas et chaussettes de compression",
        "tags",
        0x2456acf3a8524586,
    ),
    (
        "Bas et chaussettes de compression",
        "sources",
        0x5dd29e3f315adc80,
    ),
    (
        "Bande de compression à allongement court",
        "application",
        0x661be00da41c6039,
    ),
    ("Poche de colostomie", "indication", 0xe32ad0d7f49b9bb8),
    ("Poche de colostomie", "application", 0xee3b9aca81209802),
    ("Poche de colostomie", "lpp", 0xa1cd497f94c65689),
    ("Poche de colostomie", "caution", 0x21d1ea338158aec9),
    ("Poche d'iléostomie", "application", 0x285b29df59f58cb7),
    ("Poche d'iléostomie", "lpp", 0xaef19f598f8037cb),
    ("Poche d'iléostomie", "caution", 0x0fefa4a9d09c5c4c),
    ("Poche d'urostomie", "application", 0x3a7783106a3dc1bf),
    ("Poche d'urostomie", "caution", 0xb29d9fa83c90595e),
    ("Accessoires de stomie", "application", 0xf36458bc8b2e385b),
    ("Accessoires de stomie", "lpp", 0x261c186c722c0d77),
    ("Accessoires de stomie", "caution", 0x1fda7e56a9f9839e),
    ("Sonde d'autosondage", "indication", 0xee1438c6dff86c72),
    ("Sonde d'autosondage", "renewal", 0x9bf3a37dc8b3e139),
    ("Sonde d'autosondage", "caution", 0xcd494ff9d4268946),
    (
        "Étui pénien et poche de jambe",
        "application",
        0xc5cbb1a1a7aff87d,
    ),
    (
        "Étui pénien et poche de jambe",
        "caution",
        0xd1783620b600c26d,
    ),
    ("Sonde urinaire à demeure", "caution", 0xb6402036c55aa979),
    ("Protections d'incontinence", "renewal", 0xfdd4523d8193e4c1),
    ("Protections d'incontinence", "lpp", 0xb59a2e17557aa2e2),
    ("Protections d'incontinence", "caution", 0x67e1f5620c8d9fa5),
    ("Set de pansement stérile", "sizes", 0x903be650c715a5d9),
    (
        "Set de pansement stérile",
        "application",
        0xb553d842fff9c7f3,
    ),
    ("Set de pansement stérile", "lpp", 0x6d93fc1b8fd61485),
    ("Set de pansement stérile", "caution", 0x05a48fa8be9e8e09),
    (
        "Set d'ablation de fils ou d'agrafes",
        "sizes",
        0xe2a2570c460d9f1b,
    ),
    (
        "Set d'ablation de fils ou d'agrafes",
        "application",
        0x6a2f29c761ea68fa,
    ),
    (
        "Set d'ablation de fils ou d'agrafes",
        "caution",
        0x36070a4092ee073e,
    ),
    (
        "Aiguilles pour stylo à insuline",
        "indication",
        0x4df8a2eae36beba6,
    ),
    (
        "Aiguilles pour stylo à insuline",
        "sizes",
        0xce1bc8ca668893bf,
    ),
    (
        "Aiguilles pour stylo à insuline",
        "caution",
        0xec1ff703b94cd2da,
    ),
    ("Autopiqueur et lancettes", "caution", 0x86d6f3544c7a8c03),
    ("Collecteur DASRI", "caution", 0x2da091ddec58272b),
    ("Chambre d'inhalation", "indication", 0x4b0b4194492195bd),
    ("Chambre d'inhalation", "caution", 0x6db6ac62c3aba9ff),
    ("Nébuliseur (location)", "lpp", 0xfd1ad7bcb4ea9f58),
    ("Nébuliseur (location)", "caution", 0x9f9bf3780f40b7a6),
    ("Lit médicalisé (location)", "caution", 0x09324f4e6a260161),
    ("Matelas anti-escarre", "application", 0x34d0df5ce78f6117),
    ("Matelas anti-escarre", "caution", 0x5506b57c4a4df7de),
    (
        "Fauteuil roulant (location)",
        "application",
        0x8b991d2a5042bae7,
    ),
    ("Fauteuil roulant (location)", "caution", 0x6196d1cc7d68a149),
    ("Tire-lait (location)", "indication", 0x4e30091df60d853b),
    ("Tire-lait (location)", "application", 0x42ab8d408b7136b2),
    ("Tire-lait (location)", "caution", 0x1d2e8c7c18c096d0),
    ("Pansement siliconé", "indication", 0x627f946f16301aed),
    ("Pansement siliconé", "application", 0x0233edb48c5be0c7),
    ("Pansement siliconé", "lpp", 0x767fdef71f1aec7d),
    ("Pansement siliconé", "caution", 0x600e76558fda1937),
    (
        "Mèche et pansement de plaie cavitaire",
        "application",
        0xc33b85a533342ad5,
    ),
    (
        "Mèche et pansement de plaie cavitaire",
        "renewal",
        0x8a504e67b4385dfe,
    ),
    (
        "Mèche et pansement de plaie cavitaire",
        "lpp",
        0x8398b096a51cd9a9,
    ),
    (
        "Mèche et pansement de plaie cavitaire",
        "caution",
        0xc0be507230904e61,
    ),
    (
        "Pansement à l'acide hyaluronique",
        "application",
        0xe276536e1144349f,
    ),
    (
        "Pansement à l'acide hyaluronique",
        "renewal",
        0x4c32480d1631d576,
    ),
    (
        "Pansement à l'acide hyaluronique",
        "caution",
        0x3ab66481c97a3dde,
    ),
    ("Bande cohésive", "indication", 0x6e9cd6561a783ad3),
    ("Bande cohésive", "application", 0x9e4d6374749b2707),
    ("Bande cohésive", "caution", 0x023f31c389638de7),
    (
        "Manchon et gantelet du lymphœdème",
        "indication",
        0x78fcb7d9d91b46f9,
    ),
    (
        "Manchon et gantelet du lymphœdème",
        "application",
        0xd1d14e11c99d1799,
    ),
    (
        "Manchon et gantelet du lymphœdème",
        "renewal",
        0x25fd4beb7920e91c,
    ),
    (
        "Manchon et gantelet du lymphœdème",
        "caution",
        0x728f5a22c4b333e5,
    ),
    (
        "Alèse et protection de literie",
        "indication",
        0x9ca99a0f466bc71b,
    ),
    (
        "Alèse et protection de literie",
        "application",
        0x9456ab1cae605095,
    ),
    (
        "Alèse et protection de literie",
        "caution",
        0x9692a5ea7720365c,
    ),
    (
        "Poche de recueil d'urine de nuit",
        "indication",
        0x5637ae22310e45b8,
    ),
    (
        "Poche de recueil d'urine de nuit",
        "application",
        0x792171958402e3ce,
    ),
    (
        "Poche de recueil d'urine de nuit",
        "lpp",
        0xfee899099ec6382c,
    ),
    (
        "Poche de recueil d'urine de nuit",
        "caution",
        0xc4f65c61d80631bb,
    ),
    (
        "Lecteur de glycémie et bandelettes",
        "indication",
        0xbb481e8772521d35,
    ),
    (
        "Lecteur de glycémie et bandelettes",
        "application",
        0x9f367bd46d551a43,
    ),
    (
        "Lecteur de glycémie et bandelettes",
        "lpp",
        0xcb954b0e0f74a9aa,
    ),
    (
        "Lecteur de glycémie et bandelettes",
        "caution",
        0x7ae80554d284a951,
    ),
    (
        "Capteur de mesure continue du glucose",
        "indication",
        0x5137648b14f4a9f2,
    ),
    (
        "Capteur de mesure continue du glucose",
        "lpp",
        0x8ebe9151d240b050,
    ),
    (
        "Capteur de mesure continue du glucose",
        "caution",
        0x06c92c33de82bdbc,
    ),
    ("Débitmètre de pointe", "application", 0x8fd5c27834bc03af),
    ("Débitmètre de pointe", "lpp", 0x70f157563828b9ca),
    ("Débitmètre de pointe", "caution", 0x480848f12bbdd2ad),
    (
        "Concentrateur d'oxygène (location)",
        "indication",
        0x11d7b519a4657580,
    ),
    (
        "Concentrateur d'oxygène (location)",
        "application",
        0x6094cdb9337893c7,
    ),
    (
        "Concentrateur d'oxygène (location)",
        "caution",
        0xd898f3e96e260ab2,
    ),
    ("Pansement hémostatique", "application", 0xe35cfd0af73a5b54),
    ("Pansement hémostatique", "caution", 0xe53e2899c1be04dc),
    (
        "Orthèse de poignet et de pouce",
        "indication",
        0x3bcd9f1173e5fb62,
    ),
    (
        "Orthèse de poignet et de pouce",
        "application",
        0x25d0adc7e4bfec3c,
    ),
    ("Orthèse de poignet et de pouce", "lpp", 0x26a80cef1c66efbe),
    (
        "Orthèse de poignet et de pouce",
        "caution",
        0xae6726d7d4400086,
    ),
    (
        "Bas de contention anti-thrombose",
        "indication",
        0x078c5c876feb0b79,
    ),
    (
        "Bas de contention anti-thrombose",
        "application",
        0xbfe59caa83839406,
    ),
    (
        "Bas de contention anti-thrombose",
        "caution",
        0x8fed267669a45e4c,
    ),
    (
        "Poche de stomie vidable et fermée",
        "indication",
        0xf4caf111f3903aaa,
    ),
    (
        "Poche de stomie vidable et fermée",
        "application",
        0xb3e3f7f81c6696f7,
    ),
    (
        "Poche de stomie vidable et fermée",
        "caution",
        0x8230ab32020e91dc,
    ),
    (
        "Seringues et aiguilles pour injection",
        "application",
        0xc7e3bd1cb37ab9a3,
    ),
    (
        "Seringues et aiguilles pour injection",
        "renewal",
        0x0d5221ef80753946,
    ),
    (
        "Seringues et aiguilles pour injection",
        "caution",
        0x5ba16e33559f4a94,
    ),
    (
        "Nébuliseur pneumatique et son consommable",
        "renewal",
        0x586e5d13c357995c,
    ),
    (
        "Nébuliseur pneumatique et son consommable",
        "lpp",
        0xc9ba9b9840b7d38e,
    ),
    (
        "Nébuliseur pneumatique et son consommable",
        "caution",
        0xc78a64684e393aa5,
    ),
    (
        "Pilulier et aide à l'observance",
        "indication",
        0x8fe93526281927fe,
    ),
    (
        "Pilulier et aide à l'observance",
        "application",
        0xf3a43ef5b6cca079,
    ),
    (
        "Pilulier et aide à l'observance",
        "caution",
        0x963d50acf68bd5ef,
    ),
    (
        "Thermomètre et tensiomètre d'automesure",
        "indication",
        0x5feab4e868d3d700,
    ),
    (
        "Thermomètre et tensiomètre d'automesure",
        "lpp",
        0x44a64f2b6dc15abf,
    ),
    (
        "Thermomètre et tensiomètre d'automesure",
        "caution",
        0xdcd779294753be6b,
    ),
];
// StarterPreparation: 131 champs, 80 fiches anciennes, 80 nouvelles
pub const PREPARATIONS_REWORDED: &[(&str, &str, u64)] = &[
    ("Vaseline salicylée à 5 %", "caution", 0x52e8d20874189387),
    ("Vaseline salicylée à 10 %", "caution", 0x788bd2d7731f3320),
    ("Pâte à l'eau", "method", 0x48503aa69ff0c5d4),
    ("Pâte à l'eau", "conservation", 0xe841bfeeaf6301ad),
    ("Vaseline soufrée à 10 %", "caution", 0x47fc8e7f987643c5),
    (
        "Solution aqueuse d'éosine à 2 %",
        "indication",
        0x83f6c0e5490587b1,
    ),
    (
        "Solution aqueuse d'éosine à 2 %",
        "conservation",
        0xa233daf25d55bb2d,
    ),
    (
        "Solution aqueuse d'éosine à 2 %",
        "caution",
        0x645cef26877348f0,
    ),
    (
        "Gélules pédiatriques (formule type)",
        "method",
        0x3b69de5c046197a8,
    ),
    ("Sirop simple", "caution", 0x0e9cb3a52fd8f8c2),
    (
        "Solution de bicarbonate de sodium à 1,4 %",
        "caution",
        0xede17300d785d5aa,
    ),
    (
        "Solution de chlorure de sodium à 0,9 %",
        "indication",
        0xedcfd2fc0091adf7,
    ),
    ("Alcool à 70 % (V/V)", "indication", 0xc083c1bb602f099f),
    ("Alcool à 70 % (V/V)", "method", 0x6e84fbd8e5172465),
    (
        "Solution aqueuse de chlorhexidine à 0,05 %",
        "method",
        0x419f726c2a9fa806,
    ),
    ("Talc mentholé à 1 %", "caution", 0xb7eb741087a91566),
    ("Crème à l'urée à 30 %", "indication", 0x674b8bced020f8f6),
    ("Crème à l'urée à 30 %", "method", 0xf736a234f48b0ec5),
    ("Crème à l'urée à 30 %", "caution", 0x1147dd84c57a574a),
    (
        "Gel hydroalcoolique (formule OMS n° 1)",
        "indication",
        0x2d0b5f13c073ab56,
    ),
    (
        "Gel hydroalcoolique (formule OMS n° 1)",
        "caution",
        0x1954c46856d09d7a,
    ),
    (
        "Solution de chlorure de sodium à 3 %",
        "indication",
        0x81bebebdeb0bf6be,
    ),
    (
        "Solution de chlorure de sodium à 3 %",
        "caution",
        0x2e3e41819b0b011c,
    ),
    ("Glycérolé d'amidon", "method", 0x020e5d12ab2232ca),
    ("Glycérolé d'amidon", "conservation", 0xd731d93073177af9),
    ("Glycérolé d'amidon", "caution", 0xc3417a02a6ec954e),
    ("Cold cream", "indication", 0x57a132dda4c389f0),
    ("Cold cream", "method", 0xeef030c1a8bd97fe),
    ("Cold cream", "caution", 0x892a5081267fe923),
    ("Cérat de Galien", "conservation", 0x1c784f3cbacc69be),
    ("Cérat de Galien", "caution", 0x71573131d1415854),
    (
        "Solution de Dakin (hypochlorite dilué)",
        "indication",
        0xbe94d6c01912b4ec,
    ),
    (
        "Solution de Dakin (hypochlorite dilué)",
        "method",
        0x7dc89adf3f9f8a8f,
    ),
    (
        "Solution de Dakin (hypochlorite dilué)",
        "caution",
        0xa7a41f141ea75404,
    ),
    (
        "Solution alcoolique d'acide salicylique à 1 %",
        "caution",
        0x554df98370dd466d,
    ),
    (
        "Suspension buvable de nystatine (formule type)",
        "indication",
        0xae5218a85bb5a8bd,
    ),
    (
        "Suspension buvable de nystatine (formule type)",
        "caution",
        0x50b607851bfea1a4,
    ),
    ("Solution de Milian aqueuse", "method", 0x78736689639958c9),
    ("Solution de Milian aqueuse", "caution", 0x034c6f97fd79e255),
    (
        "Solution de rinçage oculaire au sérum physiologique tamponné",
        "method",
        0x19fbda88ea716822,
    ),
    (
        "Solution de rinçage oculaire au sérum physiologique tamponné",
        "caution",
        0xef46d6a784d22372,
    ),
    (
        "Gel de kétoprofène à 2,5 % (formule type)",
        "method",
        0xf017e25160a06c38,
    ),
    (
        "Gel de kétoprofène à 2,5 % (formule type)",
        "caution",
        0xc7ab0bd5a46699f7,
    ),
    (
        "Solution de morphine buvable (formule type)",
        "indication",
        0x3c078b9d1b451d06,
    ),
    (
        "Solution de morphine buvable (formule type)",
        "caution",
        0xeaeba1c3a4baff6b,
    ),
    (
        "Bain de bouche à la bétaméthasone (formule type)",
        "indication",
        0xfc1636ea2a570dbc,
    ),
    (
        "Bain de bouche à la bétaméthasone (formule type)",
        "caution",
        0x0ebdb1202bae3a76,
    ),
    (
        "Poudre orale de macrogol (formule type)",
        "caution",
        0x6c68fa7860f4ad11,
    ),
    (
        "Vaseline à l'acide borique à 3 %",
        "indication",
        0xb1d38f01d425bcc1,
    ),
    (
        "Vaseline à l'acide borique à 3 %",
        "caution",
        0x0978183e3ba5cc5a,
    ),
    ("Liniment oléo-calcaire", "indication", 0xcb5ceb072ddc0549),
    ("Liniment oléo-calcaire", "method", 0xf1d070e9e0f54cd0),
    ("Lotion à la calamine", "caution", 0xf898a0bdde306153),
    (
        "Pâte de Lassar (pâte à l'oxyde de zinc)",
        "caution",
        0x44e8c9029ab038b9,
    ),
    (
        "Solution de permanganate de potassium à 1/10 000",
        "caution",
        0xbba0df9c188056f2,
    ),
    (
        "Solution alcoolique de chlorhexidine à 0,5 %",
        "indication",
        0xdeab314bd72fba03,
    ),
    (
        "Solution de réhydratation orale (formule OMS)",
        "indication",
        0xfdd562c67fece420,
    ),
    (
        "Solution de réhydratation orale (formule OMS)",
        "caution",
        0xebfd037c446c6b1a,
    ),
    (
        "Suppositoires à excipient semi-synthétique (formule type)",
        "method",
        0xf9b2f9184e864efa,
    ),
    (
        "Pommade à la trinitrine à 0,4 % (formule type)",
        "method",
        0x2a3a6b669fe9940b,
    ),
    (
        "Pommade à la trinitrine à 0,4 % (formule type)",
        "caution",
        0xb043972784ccc330,
    ),
    (
        "Crème au métronidazole à 0,75 % (formule type)",
        "indication",
        0xf96e4c2da4420ca9,
    ),
    (
        "Crème au métronidazole à 0,75 % (formule type)",
        "caution",
        0x4ecf987e36b81bb0,
    ),
    (
        "Bain de bouche bicarbonaté",
        "indication",
        0x3cb27e9f9335ad1a,
    ),
    ("Bain de bouche bicarbonaté", "caution", 0xdfc1351d95634f02),
    (
        "Bain de bouche à l'amphotéricine B (formule type)",
        "caution",
        0xdac8323c109fb46b,
    ),
    (
        "Bain de bouche anesthésique (formule type)",
        "indication",
        0xef324e473351f42f,
    ),
    (
        "Bain de bouche anesthésique (formule type)",
        "caution",
        0x5372d7b70f93036c,
    ),
    (
        "Gouttes auriculaires à l'acide acétique",
        "indication",
        0x82d98da1eb92824c,
    ),
    (
        "Gouttes auriculaires à l'acide acétique",
        "caution",
        0x3579720423f7013c,
    ),
    ("Solution de Lugol à 1 %", "method", 0x1f7fe844001c2eb8),
    ("Solution de Lugol à 1 %", "caution", 0x862ef6ad9f176879),
    ("Pommade à l'urée à 40 %", "caution", 0x2edfed3bdd635738),
    (
        "Suspension buvable d'oméprazole (formule type)",
        "method",
        0x62405786e98ea892,
    ),
    (
        "Suspension buvable d'oméprazole (formule type)",
        "caution",
        0x2c9405c73dca03af,
    ),
    (
        "Suspension buvable de spironolactone (formule type)",
        "conservation",
        0x9dcf64eb1c6441f1,
    ),
    (
        "Suspension buvable de spironolactone (formule type)",
        "caution",
        0xd9cb1dbc10068914,
    ),
    (
        "Solution de saccharose à 24 %",
        "indication",
        0x0ea5bd879188624b,
    ),
    (
        "Solution de saccharose à 24 %",
        "caution",
        0x80611cfa5996a7a2,
    ),
    (
        "Bain de bouche fluoré à 0,05 %",
        "conservation",
        0xf7f836873e14b563,
    ),
    (
        "Bain de bouche fluoré à 0,05 %",
        "caution",
        0xef4cfd9b084b7625,
    ),
    (
        "Suspension buvable de captopril à 1 mg/mL (formule type)",
        "indication",
        0xa849b44374523de3,
    ),
    (
        "Suspension buvable de captopril à 1 mg/mL (formule type)",
        "method",
        0xf84a60555821a920,
    ),
    (
        "Suspension buvable de captopril à 1 mg/mL (formule type)",
        "conservation",
        0xeb8bfbcd6c53ab3a,
    ),
    (
        "Suspension buvable de captopril à 1 mg/mL (formule type)",
        "caution",
        0x9676f9fbe12d5b24,
    ),
    (
        "Suspension buvable de furosémide à 2 mg/mL (formule type)",
        "caution",
        0xe42a5da3ca480515,
    ),
    (
        "Suspension buvable de propranolol à 1 mg/mL (formule type)",
        "method",
        0xff9d5bf68ee05c17,
    ),
    (
        "Suspension buvable de propranolol à 1 mg/mL (formule type)",
        "caution",
        0xfa2b8748b3a36025,
    ),
    (
        "Gouttes auriculaires au bicarbonate de sodium à 5 %",
        "caution",
        0x7d2b49baee6a982d,
    ),
    (
        "Gélules de mélatonine (formule type)",
        "method",
        0xca0b0fe76fefefa0,
    ),
    (
        "Gélules de mélatonine (formule type)",
        "caution",
        0x3f83b0a1bc9090f4,
    ),
    ("Lotion mentholée à 1 %", "method", 0x347ba5583f7a965c),
    ("Lotion mentholée à 1 %", "conservation", 0x4ea3dba9e43cf695),
    ("Lotion mentholée à 1 %", "caution", 0x50929674ad645f1f),
    (
        "Solution de chlorure d'aluminium hexahydraté à 20 %",
        "indication",
        0x7da0b78947c48cbf,
    ),
    (
        "Solution de chlorure d'aluminium hexahydraté à 20 %",
        "method",
        0xf6b2f5f9e7998973,
    ),
    (
        "Solution de chlorure d'aluminium hexahydraté à 20 %",
        "caution",
        0xaba56c463f5789a3,
    ),
    (
        "Gel à l'érythromycine à 2 % (formule type)",
        "conservation",
        0x57f9465abf3791ed,
    ),
    (
        "Gel à l'érythromycine à 2 % (formule type)",
        "caution",
        0xfb401a2623a0c616,
    ),
    (
        "Solution alcoolique de clindamycine à 1 % (formule type)",
        "indication",
        0xb73bce23145472ee,
    ),
    (
        "Solution alcoolique de clindamycine à 1 % (formule type)",
        "caution",
        0x3e85a41ad5d22670,
    ),
    (
        "Pommade à l'ichtammol à 10 %",
        "caution",
        0x850a5b61a8a2569a,
    ),
    (
        "Crème au dermocorticoïde dilué au demi (formule type)",
        "indication",
        0xa063d2f6159ad3ea,
    ),
    (
        "Crème au dermocorticoïde dilué au demi (formule type)",
        "caution",
        0x2fbf5eebe13056a1,
    ),
    (
        "Poudre pour pieds au talc salicylé",
        "conservation",
        0xd7ba5867bed36366,
    ),
    (
        "Poudre pour pieds au talc salicylé",
        "caution",
        0xc08e591bdb1f718e,
    ),
    (
        "Solution d'acétate d'aluminium (solution de Burow diluée)",
        "method",
        0x946bc4d171048f5f,
    ),
    (
        "Solution d'acétate d'aluminium (solution de Burow diluée)",
        "conservation",
        0xc297b195aa28c641,
    ),
    (
        "Solution d'acétate d'aluminium (solution de Burow diluée)",
        "caution",
        0xeddaded1e58694ba,
    ),
    (
        "Solution d'eau oxygénée à 10 volumes",
        "indication",
        0x78fcf2ebf7c1048b,
    ),
    (
        "Solution d'eau oxygénée à 10 volumes",
        "conservation",
        0x5ea35cd8f2d1103d,
    ),
    (
        "Solution d'eau oxygénée à 10 volumes",
        "caution",
        0x63368d6d288bbdad,
    ),
    (
        "Solution aqueuse de bleu de méthylène à 1 %",
        "method",
        0x36b782a261faac04,
    ),
    (
        "Solution aqueuse de bleu de méthylène à 1 %",
        "caution",
        0xb736119a1b49b42a,
    ),
    (
        "Ovules à excipient semi-synthétique (formule type)",
        "conservation",
        0x0cfa4085fd4a6453,
    ),
    (
        "Ovules à excipient semi-synthétique (formule type)",
        "caution",
        0x8a75c6f4148e038c,
    ),
    (
        "Suppositoires à la glycérine (formule type)",
        "caution",
        0xcc6453cf7129c736,
    ),
    (
        "Suspension buvable d'hydrochlorothiazide à 5 mg/mL (formule type)",
        "caution",
        0x420e022c02b3047d,
    ),
    (
        "Suspension buvable d'amlodipine à 1 mg/mL (formule type)",
        "caution",
        0xbff9576ae3cc089b,
    ),
    (
        "Suspension buvable de sildénafil à 2,5 mg/mL (formule type)",
        "caution",
        0x3d84deb07a93f338,
    ),
    (
        "Suspension buvable d'acétazolamide à 25 mg/mL (formule type)",
        "caution",
        0x2489c683ded68f4d,
    ),
    (
        "Solution buvable de chlorure de potassium à 1 mmol/mL (formule type)",
        "caution",
        0xcf8d35cb32d087fe,
    ),
    (
        "Suspension buvable d'acide ursodésoxycholique à 50 mg/mL (formule type)",
        "caution",
        0x52880ac97229ad4f,
    ),
    (
        "Gélules d'acide folique à 0,4 mg (formule type)",
        "caution",
        0xc8168d1e71ce9289,
    ),
    (
        "Gélules de zinc élément à 10 mg (formule type)",
        "caution",
        0x67a0413049524e55,
    ),
    (
        "Bain de bouche lidocaïne-bicarbonate-nystatine (formule type)",
        "indication",
        0xee6ec0ead3d5a9e5,
    ),
    (
        "Bain de bouche lidocaïne-bicarbonate-nystatine (formule type)",
        "conservation",
        0xd67d366e4e98353f,
    ),
    (
        "Bain de bouche lidocaïne-bicarbonate-nystatine (formule type)",
        "caution",
        0xc1330690dda13ddd,
    ),
    (
        "Solution nasale bicarbonatée à 1,4 %",
        "method",
        0x01cbca2039b86836,
    ),
    (
        "Solution nasale bicarbonatée à 1,4 %",
        "caution",
        0x92a4f4dd07271a87,
    ),
    ("Huile gomenolée à 2 %", "caution", 0x42466c3110fad79d),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_is_fnv1a() {
        assert_eq!(fingerprint(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fingerprint("a"), 0xaf63_dc4c_8601_ec8c);
    }

    /// Chaque empreinte nomme une fiche livrée et un champ qu'elle porte :
    /// une fiche renommée ou retirée rendrait la ligne muette.
    #[test]
    fn the_reworded_fingerprints_name_existing_fields() {
        const DISPO_FIELDS: [&str; 9] = [
            "family",
            "indication",
            "sizes",
            "application",
            "renewal",
            "lpp",
            "caution",
            "tags",
            "sources",
        ];
        for (name, field, _) in DISPOSITIFS_REWORDED {
            assert!(
                crate::db::STARTER_DISPOSITIFS
                    .iter()
                    .any(|d| d.name == *name),
                "{name}"
            );
            assert!(DISPO_FIELDS.contains(field), "{field}");
        }
        const PREP_FIELDS: [&str; 9] = [
            "form",
            "indication",
            "formula",
            "yield_amount",
            "method",
            "conservation",
            "caution",
            "tags",
            "sources",
        ];
        for (name, field, _) in PREPARATIONS_REWORDED {
            assert!(
                crate::db::STARTER_PREPARATIONS
                    .iter()
                    .any(|d| d.name == *name),
                "{name}"
            );
            assert!(PREP_FIELDS.contains(field), "{field}");
        }
    }
}
