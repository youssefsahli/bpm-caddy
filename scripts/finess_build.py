# Les officines du registre FINESS, pour la carte des officines et pour
# retrouver la sienne dans Options › Officine.
#
#   python3 -I scripts/finess_build.py etalab-cs1100502-stock-*.csv assets/geo/officines.tsv
#
# Source : « FINESS Extraction du Fichier des établissements », data.gouv.fr
# (Licence Ouverte). Ne garde que la catégorie 620, pharmacie d'officine :
# numéro FINESS de l'établissement, raison sociale (longue si elle est
# écrite), code postal et ville de la ligne d'acheminement, la rue (le
# type de voie écrit en entier, en minuscules) et le téléphone.
import csv, sys
src, out = sys.argv[1], sys.argv[2]
VOIES = {
    'R': 'rue', 'AV': 'avenue', 'PL': 'place', 'BD': 'boulevard', 'RTE': 'route',
    'ALL': 'allée', 'CRS': 'cours', 'CHE': 'chemin', 'CHEM': 'chemin', 'QU': 'quai',
    'QUA': 'quai', 'LD': 'lieu-dit', 'CCAL': 'centre commercial', 'IMP': 'impasse',
    'SQ': 'square', 'LOT': 'lotissement', 'RES': 'résidence', 'ESP': 'esplanade',
    'ESPA': 'esplanade', 'FG': 'faubourg', 'RPT': 'rond-point', 'GR': 'grande rue',
    'VOI': 'voie', 'PAS': 'passage', 'CTRE': 'centre', 'PROM': 'promenade',
    'CAR': 'carrefour', 'CARR': 'carrefour', 'TRA': 'traverse', 'GAL': 'galerie',
    'CHS': 'chaussée', 'MAIL': 'mail', 'COR': 'corniche', 'CITE': 'cité',
    'DOM': 'domaine', 'MTE': 'montée', 'IMM': 'immeuble', 'HAM': 'hameau',
    'VGE': 'village', 'PTE': 'porte', 'BRG': 'bourg', 'PLAN': 'plan', 'PARC': 'parc',
    'CLOS': 'clos', 'ZONE': 'zone', 'TOUR': 'tour', 'COUR': 'cour', 'GARE': 'gare',
    'PLAG': 'plage', 'VAL': 'val', 'STA': 'station', 'ARC': 'arcades', 'PRE': 'pré',
}
SUITE = {'B': 'bis', 'T': 'ter', 'Q': 'quater'}
def words(*parts):
    return ' '.join(' '.join(p.split()) for p in parts if p and p.strip())
rows = []
for r in csv.reader(open(src, encoding='utf-8'), delimiter=';'):
    if r[0] != 'structureet' or r[18] != '620':
        continue
    nom = words(r[4] or r[3])
    cp, _, ville = r[15].partition(' ')
    ville = words(ville)
    assert len(r[1]) == 9 and len(cp) == 5, r
    num = words(r[7], SUITE.get(r[10].strip().upper(), r[10]))
    rue = words(num, VOIES.get(r[8].strip().upper(), r[8]), r[9])
    lieu = words(r[11])
    # Une boîte postale n'est pas une adresse.
    if lieu and not lieu.upper().startswith(('BP', 'CS ', 'B.P')):
        rue = f'{rue}, {lieu}' if rue else lieu
    tel = ''.join(c for c in r[16] if c.isdigit())
    for v in (nom, ville, rue):
        assert '\t' not in v
    rows.append(f'{r[1]}\t{nom}\t{cp}\t{ville}\t{rue}\t{tel}')
rows.sort()
with open(out, 'w', encoding='utf-8') as fh:
    fh.write('\n'.join(rows) + '\n')
print(len(rows), 'officines')
