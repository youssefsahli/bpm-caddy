# Geographic data

- `departements.tsv` — département outlines, simplified (Douglas–Peucker,
  0.01° in metropolitan France, 0.004° overseas), coordinates rounded to
  0.001°. Source: IGN Admin Express COG 2018 and INSEE Code officiel
  géographique, through <https://github.com/gregoiredavid/france-geojson>.
  Licence Ouverte / Open Licence (Etalab).
- `communes.tsv` — one line per commune and postcode: postcode, name,
  centre longitude and latitude, département, population. Source:
  <https://geo.api.gouv.fr/communes> (INSEE, La Poste, IGN), fetched
  2026-10-12. Licence Ouverte / Open Licence (Etalab).

Regenerate with `scripts/geo_build.py <geojson dir> <communes.json> assets/geo`
(the geojson directory holds `deps.geojson`, the simplified metropolitan
file, and `dom-*.geojson`, one per overseas département).
