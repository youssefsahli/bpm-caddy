# Le compagnon Android — construire, signer, publier

Ce que le téléphone tient et pourquoi : `docs/SYNC.md` § 7.8. Ce
document dit comment on le construit et ce qu'il faut pour le Play
Store.

## Construire

Une fois par machine :

```
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk
# SDK Android (ANDROID_HOME) avec platform 36 et build-tools 36 ;
# NDK r27 dans $ANDROID_HOME/ndk/ ; un JDK 17 ou 21 pour Gradle.
```

Puis :

```
cd android && ./gradlew assembleDebug      # un APK d'essai
cd android && ./gradlew bundleRelease      # l'AAB pour le Play Store
```

Gradle lance lui-même `build-rust.sh` (tâche `buildRust`) dès qu'une
source Rust, `Cargo.lock` ou `assets/strings.fr.toml` a changé : **les
libellés du téléphone sont compilés dans la bibliothèque**, et un APK
construit sur une bibliothèque d'avant montrait les clés (`mobile_…`) à
la place du français. Les liaisons Kotlin se tirent toujours de la
version de débogage de l'hôte — le profil release de l'atelier retire
les symboles, et le générateur n'écrirait rien.

`build-rust.sh --debug` va plus vite pour essayer, mais la bibliothèque
de débogage pèse 200 Mo et synchronise lentement ; la version release
pèse 8 Mo. Les deux dossiers écrits (`app/src/main/jniLibs`,
`app/src/main/java/uniffi`) ne sont pas suivis par git.

L'intégration continue (`.github/workflows/ci.yml`, tâche `android`)
construit l'APK de débogage à chaque poussée.

## Signer

La clé de publication **ne vit pas dans le dépôt**. Gradle la lit dans
`android/keystore.properties` (ignoré par git) :

```
storeFile=/chemin/vers/bpm-caddy-play.jks
storePassword=…
keyAlias=bpm-caddy
keyPassword=…
```

ou dans les variables `BPM_ANDROID_STORE_FILE`,
`BPM_ANDROID_STORE_PASSWORD`, `BPM_ANDROID_KEY_ALIAS`,
`BPM_ANDROID_KEY_PASSWORD`. Sans elles, `bundleRelease` produit un AAB
non signé. Avec la signature d'application Play, cette clé est la *clé
d'importation* : Google garde la clé de signature. La créer :

```
keytool -genkeypair -v -keystore bpm-caddy-play.jks -keyalg RSA \
    -keysize 4096 -validity 10000 -alias bpm-caddy
```

**À conserver hors de la machine** (coffre, copie chiffrée) : perdue,
elle se réinitialise auprès de Google, mais pas sans délai.

## Identité

- Identifiant : `io.github.youssefsahli.bpmcaddy` — définitif une fois
  publié.
- Version : `versionName` suit l'application de bureau, `versionCode` =
  mineure × 1000 (0.351.0 → 351000) ; à monter avec les trois crates.
- Android 8 (API 26) au minimum, cible API 36.

## Fiche du Play Store (proposition)

**Titre** : BPM-Caddy — compagnon d'officine

**Description courte** : Les fiches, l'agenda et la messagerie de
l'officine, sur le téléphone de l'équipe.

**Description** :

> Le compagnon de BPM-Caddy, le logiciel de pharmacie clinique de
> l'officine. Relié à un poste de l'officine par un code à usage
> unique, le téléphone d'un membre de l'équipe reçoit :
>
> - les fiches médicaments de l'officine, avec recherche par nom, DCI ou
>   classe, et leur correction ;
> - l'agenda de l'officine, en lecture et en écriture ;
> - le planning de l'équipe ;
> - la messagerie de l'équipe.
>
> Aucun serveur : le téléphone parle directement aux postes de
> l'officine, sur le Wi-Fi ou par Internet quand un poste est rendu
> joignable. Tout ce qui voyage est chiffré de bout en bout. Le
> téléphone ne reçoit jamais la clé des dossiers patients, du registre
> des stupéfiants ni de la caisse. Sa base est chiffrée et ne s'ouvre
> qu'avec le verrouillage du téléphone.
>
> Nécessite BPM-Caddy sur un poste de l'officine.

## Sécurité des données (formulaire Play)

Réponses à vérifier par l'officine avant de les soumettre :

- **Données collectées par le développeur** : aucune. L'application
  n'envoie rien à un serveur du développeur ni d'un tiers ; elle n'a
  ni publicité, ni mesure d'audience, ni rapport de plantage.
- **Données partagées** : aucune avec des tiers. Les données voyagent
  entre les appareils de l'officine, chiffrées de bout en bout, sous le
  contrôle de l'officine.
- **Chiffrement en transit** : oui (Noise XX, puis XChaCha20-Poly1305
  par enregistrement).
- **Suppression** : désinstaller l'application supprime la base ; un
  poste peut retirer le téléphone du groupe.
- **Catégorie santé — à déclarer** : les fiches médicaments ne sont pas
  des données personnelles, mais la messagerie d'équipe peut en porter :
  un message peut citer un numéro de dossier patient (le téléphone n'a
  pas la clé du dossier lui-même) et un fichier joint peut être un
  document de patient envoyé par l'équipe. Déclarer « informations de
  santé » comme données *traitées sur l'appareil*, non collectées par
  le développeur.
- **Autorisations** : `INTERNET`, `ACCESS_WIFI_STATE`,
  `CHANGE_WIFI_MULTICAST_STATE` (entendre les annonces des postes sur
  le Wi-Fi), `USE_BIOMETRIC` (ouvrir la base).

Une **politique de confidentialité** publique est exigée : le texte
ci-dessous peut être publié tel quel (page GitHub du dépôt, par
exemple).

## Politique de confidentialité (proposition)

> **BPM-Caddy — compagnon Android**
>
> L'application ne collecte aucune donnée pour son éditeur et n'en
> transmet à aucun tiers. Elle ne contient ni publicité, ni outil de
> mesure d'audience, ni rapport d'erreur envoyé à distance.
>
> Les données affichées (fiches médicaments, agenda, planning,
> messagerie de l'équipe et ses fichiers joints, qui peuvent concerner
> des patients) proviennent des postes de l'officine auxquels
> l'utilisateur a relié son téléphone, et n'en sortent que vers ces
> mêmes postes, chiffrées de bout en bout. Elles sont conservées sur le
> téléphone dans une base chiffrée, dont la clé est protégée par le
> magasin de clés Android et le verrouillage de l'écran.
>
> Le téléphone ne reçoit jamais la clé permettant de lire les dossiers
> patients, le registre des stupéfiants ou la caisse de l'officine.
>
> Désinstaller l'application efface ses données. Pour toute question :
> l'officine responsable du traitement, ou l'éditeur via le dépôt du
> projet.
