-- The database schema as v0.109.0 shipped it, kept as a fixture.
--
-- It is not maintained: it is a photograph. A base created by that
-- version still exists in officines, and the only thing that turns it
-- into a base this version can read is the MIGRATIONS list. The test
-- that opens this file is what says so out loud.

CREATE TABLE IF NOT EXISTS patients (
    id          INTEGER PRIMARY KEY,
    last_name   TEXT NOT NULL,
    first_name  TEXT NOT NULL,
    birth_date  TEXT NOT NULL,
    phone       TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    physician   TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    situation   TEXT NOT NULL DEFAULT '',
    nir         TEXT NOT NULL DEFAULT '',
    regime      TEXT NOT NULL DEFAULT '',
    -- Local time, like every other stamp in this base: the counter
    -- works in its own clock, and an act entered after midnight in a
    -- pharmacie de garde must carry that day, not the UTC one.
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS interviews (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'IDENTIFIED',
    duration_minutes INTEGER NOT NULL DEFAULT 0,
    scheduled_date TEXT,
    scheduled_time TEXT NOT NULL DEFAULT '',
    remote      INTEGER NOT NULL DEFAULT 0,
    treatment_change INTEGER NOT NULL DEFAULT 0,
    theme       TEXT NOT NULL DEFAULT '',
    trod_result TEXT NOT NULL DEFAULT '',
    operator    TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS drugs (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    dci         TEXT NOT NULL DEFAULT '',
    class       TEXT NOT NULL DEFAULT '',
    dosage      TEXT NOT NULL DEFAULT '',
    ddi         TEXT NOT NULL DEFAULT '',
    iup         TEXT NOT NULL DEFAULT '',
    indications TEXT NOT NULL DEFAULT '',
    mechanism   TEXT NOT NULL DEFAULT '',
    contraindications TEXT NOT NULL DEFAULT '',
    adverse     TEXT NOT NULL DEFAULT '',
    monitoring  TEXT NOT NULL DEFAULT '',
    sources     TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT '',
    smr         TEXT NOT NULL DEFAULT '',
    tags        TEXT NOT NULL DEFAULT '',
    toxicity    TEXT NOT NULL DEFAULT '',
    forms       TEXT NOT NULL DEFAULT '',
    antidote    TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    half_life   TEXT NOT NULL DEFAULT '',
    auc         TEXT NOT NULL DEFAULT '',
    elimination TEXT NOT NULL DEFAULT '',
    renal       TEXT NOT NULL DEFAULT '',
    pregnancy   TEXT NOT NULL DEFAULT '',
    -- Conduite à tenir en cas d'oubli, et les signes qui font consulter.
    missed_dose TEXT NOT NULL DEFAULT '',
    red_flags   TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS drug_field_locks (
    drug_id     INTEGER NOT NULL REFERENCES drugs(id),
    column_name TEXT NOT NULL,
    PRIMARY KEY (drug_id, column_name)
);
CREATE TABLE IF NOT EXISTS protocols (
    id          INTEGER PRIMARY KEY,
    title       TEXT NOT NULL,
    subject     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS protocol_nodes (
    id          INTEGER PRIMARY KEY,
    protocol_id INTEGER NOT NULL,
    parent_id   INTEGER,
    branch      TEXT NOT NULL DEFAULT 'ROOT',
    kind        TEXT NOT NULL DEFAULT 'ACTION',
    text        TEXT NOT NULL DEFAULT '',
    position    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS posologies (
    id          INTEGER PRIMARY KEY,
    drug_id     INTEGER NOT NULL REFERENCES drugs(id),
    indication  TEXT NOT NULL,
    posologie   TEXT NOT NULL DEFAULT '',
    remarque    TEXT NOT NULL DEFAULT '',
    position    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS biology (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    -- Code de l'analyte dans `crate::biology`, vide pour une ligne
    -- écrite à la main.
    code        TEXT NOT NULL DEFAULT '',
    label       TEXT NOT NULL,
    value       REAL NOT NULL DEFAULT 0,
    unit        TEXT NOT NULL DEFAULT '',
    -- Date du prélèvement, ISO.
    taken_on    TEXT NOT NULL DEFAULT '',
    remark      TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS locations (
    id           INTEGER PRIMARY KEY,
    patient_id   INTEGER NOT NULL REFERENCES patients(id),
    -- Le libellé du forfait, recopié depuis `[locations]` au moment de
    -- la pose : le tarif de la configuration peut changer, ce que le
    -- patient a signé, non.
    label        TEXT NOT NULL,
    lpp          TEXT NOT NULL DEFAULT '',
    -- 'jour', 'semaine' ou 'mois'.
    period       TEXT NOT NULL DEFAULT 'semaine',
    fee          REAL NOT NULL DEFAULT 0,
    renewal_days INTEGER NOT NULL DEFAULT 0,
    max_periods  INTEGER NOT NULL DEFAULT 0,
    -- Jour de pose et jour de reprise, ISO ; reprise vide = en cours.
    started_on   TEXT NOT NULL DEFAULT '',
    ended_on     TEXT NOT NULL DEFAULT '',
    -- Dernier renouvellement d'ordonnance enregistré, ISO ; vide = le
    -- compte part de la pose.
    renewed_on   TEXT NOT NULL DEFAULT '',
    remark       TEXT NOT NULL DEFAULT ''
);
-- Ce que les contenus livrés ont déjà semé dans cette base : une
-- ligne par graine, pour ne pas réécrire à chaque lancement ce qui
-- l'est déjà.
CREATE TABLE IF NOT EXISTS seed_state (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS preparations (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    -- Forme galénique : pommade, gélules, solution…
    form         TEXT NOT NULL DEFAULT '',
    indication   TEXT NOT NULL DEFAULT '',
    -- Une ligne par ingrédient : « nom | quantité unité ».
    formula      TEXT NOT NULL DEFAULT '',
    -- Ce que cette formule produit, unité comprise.
    yield_amount TEXT NOT NULL DEFAULT '',
    method       TEXT NOT NULL DEFAULT '',
    conservation TEXT NOT NULL DEFAULT '',
    caution      TEXT NOT NULL DEFAULT '',
    tags         TEXT NOT NULL DEFAULT '',
    sources      TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS dispositifs (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    -- Famille : pansement, compression, stomie, sondage, location…
    family       TEXT NOT NULL DEFAULT '',
    indication   TEXT NOT NULL DEFAULT '',
    -- Formes, tailles, présentations.
    sizes        TEXT NOT NULL DEFAULT '',
    -- La pose : ce qui va dessous, la découpe, le geste.
    application  TEXT NOT NULL DEFAULT '',
    -- Rythme de renouvellement, durée de port, quantité par mois.
    renewal      TEXT NOT NULL DEFAULT '',
    -- Ligne LPP et prise en charge, telles que l'équipe les vérifie.
    lpp          TEXT NOT NULL DEFAULT '',
    caution      TEXT NOT NULL DEFAULT '',
    tags         TEXT NOT NULL DEFAULT '',
    sources      TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS class_notes (
    class       TEXT PRIMARY KEY,
    body        TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS table_cells (
    table_key   TEXT NOT NULL,
    row         INTEGER NOT NULL,
    col         INTEGER NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    PRIMARY KEY (table_key, row, col)
);
CREATE TABLE IF NOT EXISTS events (
    id          INTEGER PRIMARY KEY,
    day         TEXT NOT NULL,
    time        TEXT NOT NULL DEFAULT '',
    end_time    TEXT NOT NULL DEFAULT '',
    repeat_days INTEGER NOT NULL DEFAULT 0,
    repeat_until TEXT NOT NULL DEFAULT '',
    title       TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'AUTRE',
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS patient_drugs (
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    drug_id     INTEGER NOT NULL REFERENCES drugs(id),
    PRIMARY KEY (patient_id, drug_id)
);
CREATE TABLE IF NOT EXISTS notes (
    id           INTEGER PRIMARY KEY,
    subject_kind TEXT NOT NULL,
    subject_id   INTEGER NOT NULL DEFAULT 0,
    operator     TEXT NOT NULL DEFAULT '',
    body         TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS vaccinations (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    code        TEXT NOT NULL DEFAULT '',
    label       TEXT NOT NULL,
    dose        TEXT NOT NULL DEFAULT '',
    given_on    TEXT NOT NULL DEFAULT '',
    lot         TEXT NOT NULL DEFAULT '',
    site        TEXT NOT NULL DEFAULT '',
    operator    TEXT NOT NULL DEFAULT '',
    next_due    TEXT NOT NULL DEFAULT '',
    remark      TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS patient_travel (
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    country     TEXT NOT NULL,
    depart_on   TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (patient_id, country)
);
