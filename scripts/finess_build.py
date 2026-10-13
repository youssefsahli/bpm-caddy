# Les officines du registre FINESS, pour la carte des officines.
#
#   python3 -I scripts/finess_build.py etalab-cs1100502-stock-*.csv assets/geo/officines.tsv
#
# Source : « FINESS Extraction du Fichier des établissements », data.gouv.fr
# (Licence Ouverte). Ne garde que la catégorie 620, pharmacie d'officine :
# numéro FINESS de l'établissement, raison sociale (longue si elle est
# écrite), code postal et ville de la ligne d'acheminement.
import csv, sys
src, out = sys.argv[1], sys.argv[2]
rows = []
for r in csv.reader(open(src, encoding='utf-8'), delimiter=';'):
    if r[0] != 'structureet' or r[18] != '620':
        continue
    nom = ' '.join((r[4] or r[3]).split())
    cp, _, ville = r[15].partition(' ')
    ville = ' '.join(ville.split())
    assert len(r[1]) == 9 and len(cp) == 5, r
    rows.append(f'{r[1]}\t{nom}\t{cp}\t{ville}')
rows.sort()
with open(out, 'w', encoding='utf-8') as fh:
    fh.write('\n'.join(rows) + '\n')
print(len(rows), 'officines')
