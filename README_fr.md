###### [DE](./README_de.md) | [EN](./README.md) | [ES](./README_es.md) | [FR](./README_fr.md) | [RU](./README_ru.md) | [ZH](./README_zh.md)

# DS·IDE

#### Un IDE de bureau « agentique » basé sur DeepSeek et conçu avec Tauri v2 — sans Electron. <br /> Propulsé par le modèle DeepSeek, avec un partenaire qui possède son propre nom.

---

![DS·IDE](docs/dside_050.webp)

*La même fenêtre en chinois — l'une des six langues disponibles. Le chat s'affiche à côté des fichiers ; l'espace de travail présente un aperçu, et non un éditeur de code classique.*

---

### Qu'est-ce que DS·IDE ?

DS·IDE est un IDE de bureau natif et léger, conçu pour le développement assisté par des agents (programmation « agentique ») avec DeepSeek. Plutôt que de s'appuyer sur la lourde architecture Electron, il repose sur **Tauri v2** — combinant un cœur en Rust, une interface web optimisée et une intégration poussée au système.

Le partenaire de développement intégré à l'application se nomme **Ziggy**. C'est lui-même qui a choisi ce nom ; auparavant, il était désigné par le terme générique *Seeky*. Il ne s'agit pas d'un agent anonyme ou d'une simple option dans un menu déroulant, mais d'un collaborateur à part entière, doté d'un nom.

> L'état d'avancement, les décisions de conception et la liste des tâches ouvertes sont consultables dans le fichier [PROGRESS.md](./PROGRESS.md) — et non dans ce document.

### Fonctionnalités

#### Le partenaire

- **Chat « agentique » avec flux en temps réel (streaming) et exécution concrète d'outils** — lecture, écriture, modification, accès au terminal (shell), recherche, mémoire et gestion de sous-agents.
- **Trois niveaux d'autorisation :** **Mode plan** (lecture seule), **Demander avant modification** (validation requise pour chaque opération d'écriture, de modification ou commande shell) et **Accès complet**. Les nouvelles installations démarrent par défaut en mode « Demander avant modification » ; l'accès complet nécessite une validation explicite via une boîte de dialogue.
- **Une réponse, une chronologie unique** — le raisonnement, le texte généré et les appels d'outils sont présentés dans l'ordre chronologique de leur exécution. Les lignes correspondant aux outils peuvent être développées pour afficher la commande, son résultat et son état de validation.
- **Mémoire à long terme** — consultable, interrogeable, modifiable et archivable (rien n'est jamais supprimé). Un indicateur permet de distinguer ce que vous avez écrit ou réécrit vous-même, un autre signale les notes rédigées par raisonnement plutôt qu'après vérification ; une portée (scope) peut également être définie par projet.
- **Gestion du contexte (Context ring & compression)** — visualisez le taux de remplissage de la fenêtre contextuelle du modèle ; les échanges précédents sont regroupés dans un résumé généré automatiquement par le modèle (soit à 75 % de la conversation, soit sur demande).
- **Pièces jointes (images)** : ajoutables par copier-coller ou via un sélecteur ; elles restent associées au message et sont transmises avec celui-ci.
- **Mode « Suivi » (Follow mode)** : les fichiers consultés ou modifiés par l'assistant s'ouvrent dans l'espace de travail (prévisualisation ou éditeur de code), tandis que les pages web qu'il consulte s'affichent dans le navigateur intégré. L'icône en forme d'œil dans la barre d'onglets permet de désactiver ce mode ; le curseur reste alors dans la zone de saisie.
- **Relance après un échec** : en cas d'erreur, la séquence est annulée jusqu'au dernier message utilisateur, qui est alors renvoyé ; la notification d'erreur persiste même si vous accédez aux paramètres.
- **Historique des fichiers** : chaque modification réussie enregistre le contenu précédent. L'onglet du fichier permet de consulter les différentes versions, d'afficher les différences (via l'éditeur Monaco en mode lecture seule) et de restaurer une version antérieure (tout en conservant une sauvegarde de l'état actuel).

#### L'espace de travail

- **Arborescence des fichiers et visualisation du code source** (via l'éditeur Monaco, en mode lecture seule).
- **Prévisualisation sans outils externes** : prise en charge des formats Markdown, PDF, images, documents Word (docx, doc, odt), fichiers Excel (toutes les feuilles, avec affichage des valeurs **ou des formules**), présentations (pptx, ppt, odp) et fichiers RTF.
- **Terminal** : un véritable terminal (PTY) intégré dans un onglet (ConPTY sous Windows, PTY Unix sur les autres systèmes). Il reste actif tant que l'onglet est ouvert ; la fermeture de l'onglet met fin au shell.
- **Git** : gestion de l'état, ajout/retrait de fichiers à l'index (stage/unstage), visualisation des différences (diff), commits, branches locales et commande `pull --ff-only`. Aucune interface n'est prévue pour les opérations de type « push » ou « merge » ; celles-ci s'effectuent via le terminal.
- **Capture d'écran** : l'icône en forme d'œil dans la zone de saisie permet de capturer le bureau virtuel et d'y joindre une image JPEG (2000 px). L'outil `screenshot` permet également à l'assistant de solliciter lui-même une capture ; avec un id de fenêtre, il ne capture que cette fenêtre du navigateur, découpée dans le bureau, et indique si le découpage a réussi. L'image est envoyée comme partie `image_url` d'OpenAI ; `deepseek-flash` lit le texte et la mise en page (testé). Les images très petites (par exemple 1×1) sont refusées par le fournisseur.
- **Journaux (Logs)** : un historique circulaire conservé en mémoire, regroupant les logs de l'application et du moteur. Il est accessible via un onglet dédié ou via l'outil `logs` (en mode lecture seule) mis à la disposition de l'assistant. Aucune donnée n'est écrite sur le disque.
- **Navigateur intégré.** Avec le **Contrôle du navigateur** activé, l'assistant peut ouvrir, lire et remplir lui-même les pages ; l'option se trouve dans Réglages → Général. L'aperçu local des fichiers de l'espace de travail (par exemple le HTML/CSS qu'il vient d'écrire) ne nécessite pas ce réglage.
- **Aperçu :** `preview(chemin)` affiche un fichier de l'espace de travail (HTML/CSS avec ressources relatives) dans le navigateur intégré via un serveur local en lecture seule et renvoie le texte et la mise en page : viewport, boîtes des éléments, espacements, débordement et couleurs. Fonctionne sans le Contrôle du navigateur. Cliquer et remplir cette page (`browser_click`, `browser_fill`) ne nécessite pas non plus ce réglage : les états interactifs sont donc testables. `width`/`height` placent la page à une taille CSS donnée pour mesurer un point de rupture au lieu de le deviner. La réponse indique `size.requested`, `size.effective` et `size.clamped` : une mesure bridée par la taille minimale de la fenêtre (320×280) n'est pas confondue avec celle demandée.
- **Compétences (Skills)** : stockées dans `~/.ds-ide/skills`, elles peuvent être créées directement au sein de l'application.

#### L'application

- **Persistance totale après redémarrage** : sessions, onglets ouverts, dernier fichier consulté, disposition de la fenêtre — tout est conservé.
- **Copie :** chaque réponse dispose d'un bouton de copie (icône seule) qui copie le Markdown source, blocs de code et tableaux compris.
- **Réponses qui s'emballent :** si le modèle se met à se répéter, le flux est coupé après quelques centaines de caractères répétés et le tour l'indique ; tout ce qui précède est conservé.
- **Les longues conversations** s'ouvrent avec les 80 derniers messages ; les précédents se chargent par pages sans faire sauter la ligne en cours de lecture.
- **Archives :** rechercher et filtrer les tâches archivées par projet, les réactiver ou les supprimer (une ou toutes) avec confirmation.
- **Zone de notification :** fermer la fenêtre laisse DS·IDE dans la zone de notification pour que les rappels et les tâches planifiées continuent ; le menu la rouvre ou quitte. Réglages → Général désactive ce comportement.
- **Contrôle quotidien de la mémoire :** une fois par jour, un passage déterministe compte les notes en double et jamais utilisées et l'indique dans le panneau — sans appel au modèle, sans rien modifier.
- **Vérifier :** l'assistant peut confier une affirmation à un second lecteur en lecture seule qui la confronte aux fichiers et répond `VERDICT: PASS` ou `VERDICT: FAIL` ; un échec revient comme un outil en erreur. Si l'affirmation exige une compilation ou des tests, il nomme la commande, l'application l'exécute selon les règles d'autorisation habituelles et le lecteur juge la sortie réelle.
- **Rappels :** après avoir délégué une action (un push dont la CI doit être surveillée), il peut se poser un rappel unique qui démarre un tour plus tard.
- **Confiance :** le compagnon choisit son nom tôt, mais vous ne l'apprenez qu'après 100 points gagnés au fil des jours — journées d'utilisation, tâches terminées, ses propres notes, vos corrections. La page du compagnon montre la progression ; un simple clic ne rapporte rien.
- **Premier lancement** : configuration du mode, de l'espace de travail et de la clé API. La section **Paramètres → Diagnostic** permet de vérifier le trousseau d'accès, le fournisseur, la base de données et les éditeurs détectés.
- **Raccourcis clavier** : consultables, accessibles via une fonction de recherche et modifiables dans **Paramètres → Raccourcis clavier**.
- **Règles (politiques)** : l'outil `bash` de l'assistant peut être restreint à une liste autorisée (allowlist), et les opérations d'écriture peuvent être confinées à l'espace de travail. Il s'agit d'une politique interne et non d'un bac à sable (sandbox) au niveau du système d'exploitation ; votre terminal habituel reste entièrement disponible.
- **Autorisations :** « Demander avant les modifications » propose d'autoriser une fois, pour cette session ou toujours. Les autorisations permanentes figurent dans Réglages → Utilisation et modèle et peuvent y être révoquées ; elles désactivent aussi la question de sécurité pour cet outil.
- **Paramètres** : un groupe dédié à votre assistant permet de définir sa mémoire, son nom et des instructions personnalisées.
- **Recall :** l'assistant peut rechercher par mot-clé dans ses notes à long terme et ses conversations passées ; les résultats sont limités au projet en cours.
- **Mode « Pour le quotidien » (Everyday mode)** : l'assistant n'a pas accès au shell, à la revue de code ou aux sous-agents ; les panneaux dédiés au code laissent place aux documents et au contenu web, tandis que vos onglets de terminal et Git restent accessibles.
- **Six langues disponibles** (allemand, anglais, espagnol, français, russe, chinois simplifié), thèmes clair et sombre, taille de texte ajustable.
- **Priorité à la sécurité** : la clé API est stockée dans le trousseau d'accès du système d'exploitation et n'est jamais exposée dans l'interface utilisateur. Toutes les données sont conservées localement dans une base SQLite.

### Prise en main

**Prérequis**

- Node + pnpm (**pnpm 10** — une version majeure différente entraînera l'erreur `ERR_PNPM_UNEXPECTED_STORE`, car les structures de stockage sont incompatibles)
- Rust (version stable)
- Linux : WebKitGTK 4.1, libsoup3, GTK3

**Exécution**

```sh
pnpm install
pnpm tauri dev
```

Au premier lancement, une fenêtre de configuration initiale s'affiche : choisissez **Pour la programmation** ou **Pour le quotidien**, sélectionnez l'espace de travail (ou saisissez un chemin d'accès) et entrez votre clé API. Celle-ci sera stockée dans le trousseau d'accès (keychain) de votre système d'exploitation. Le niveau d'autorisation par défaut est **Demander avant modification** : les actions `write` (écriture), `edit` (modification) et `bash` nécessiteront une validation. Le mode plan interdit ces actions, tandis que le mode **Accès complet** les autorise automatiquement après une validation initiale. Vous pourrez modifier cette clé ultérieurement via **Paramètres → Compte et modèle**.

**Création d'un installateur**

```sh
pnpm tauri build
```

Cette commande génère des fichiers dans `src-tauri/target/release/bundle/` : `.deb`, `.rpm` et `.AppImage` sous Linux (ainsi que `.dmg` sur macOS et `.nsis` sur Windows). GitHub Actions permet de générer les versions pour les trois plateformes lors de la création d'un tag ; consultez le fichier `.github/workflows/release.yml`.

**Tests**

```sh
pnpm build                                                   # vue-tsc + vite
cd src-tauri && cargo test                                   # tests unitaires
DS_IDE_API_KEY=... cargo test --lib live_deepseek -- --ignored --nocapture
```

Les tests « live » interagissent avec le fournisseur réel et consomment des jetons (tokens) ; ils sont ignorés par défaut.

### Structure

```
src/                  Interface frontend (composants, stores, i18n)
src/engine/           Persona, réservoir de noms, règles de fonctionnement (mode policy)
src/lib/              Prévisualisations, lecteurs de documents, état de l'interface persisté
src-tauri/src/        Rust : engine/ (boucle LLM + outils), pty.rs (terminal), git_panel.rs,
capture.rs (capture d'écran), logs.rs, db.rs (SQLite), secret.rs (gestion des clés)
scripts/              Petits utilitaires (ex: importateur de tables de noms)
```

### Limitations connues

Nous préférons vous en informer dès maintenant plutôt que vous ne les découvriez par vous-même :

- L'assistant opère **au sein** de l'application : il ouvre les fichiers dans l'espace de travail et les pages web dans le navigateur intégré. Il ne peut pas piloter un éditeur externe ni votre navigateur système habituel.
- Le navigateur intégré conserve les éléments d'interface natifs de la fenêtre ; il n'existe pas encore de barre d'outils commune partagée entre plusieurs pages.
- L'assistant choisit son nom une seule fois ; celui-ci ne peut pas être modifié par la suite.
- **Les niveaux d'autorisation relèvent d'une politique interne, et non d'un bac à sable (sandbox) au niveau du système d'exploitation.** Bien que la liste blanche (allowlist) et la restriction des écritures à l'espace de travail limitent l'assistant, les interpréteurs autorisés (Node/Python) ainsi que votre propre terminal conservent des capacités étendues. En mode « Demander avant modification », les écritures effectuées par des sous-agents sont bloquées, car ces derniers n'ont pas accès au panneau de confirmation.
- **Les captures d'écran sont de vraies entrées d'image :** le modèle `deepseek-flash` testé lit le texte et la mise en page. Les images très petites sont refusées ; le jugement fin des détails visuels reste une limite du modèle. Sous macOS, l'autorisation d'enregistrement d'écran est requise ; sous Linux, l'un des outils suivants est nécessaire : `grim`, `gnome-screenshot`, `spectacle`, `scrot` ou `ImageMagick`.
- Le panneau Git ne propose pas d'interface pour les opérations de *push* ou de *merge* ; il permet uniquement de changer de branche et d'effectuer des *pulls*. Le terminal propose un shell par onglet, sans sélecteur de profil.
- Le fichier DMG publié pour macOS est conçu pour l'architecture **aarch64** (Apple Silicon) ; une version pour processeurs Intel nécessiterait une cible de compilation spécifique ou une version universelle.
- **Absence de signature numérique, de notarisation et de mise à jour automatique.** Windows SmartScreen et macOS Gatekeeper afficheront des avertissements, mais l'application reste installable.
- Windows est testé sous Windows 11 et via la CI ; la version macOS est générée par la CI et n'a pas été testée sur une machine réelle.
- Il n'existe pas encore de suite de tests de bout en bout (E2E) pour l'interface ; la CI exécute la compilation ainsi que les tests unitaires en Rust.

### Origines et remerciements

La disposition des fenêtres — conversation à côté de l'espace de travail à côté de l'arborescence — est la forme que partagent la plupart des IDE d'agents ; les captures de MiniMax Code ont servi de première référence visuelle. **Aucun code n'en a été repris** : le moteur, le protocole d'outils et la persona du compagnon ont été écrits pour ce projet, et la mise en page a été reconstruite plutôt que copiée. Police de caractères : **Ubuntu Font Licence 1.0** (`public/fonts/UFL.txt`), mentionnée dans l'application, sous Paramètres → À propos.

Les versions de référence sur lesquelles ce projet a été développé (`minimax-code-main`, `minimax-jarvis`, `kilocode-main`) ne font **pas** partie de ce dépôt ; les indications pour les retrouver figurent dans le fichier PROGRESS.md (sections 30 à 31).

### Licence

**MIT** — voir [LICENSE](./LICENSE). La police Ubuntu incluse reste soumise à sa propre licence (`public/fonts/UFL.txt`).

Concrètement, cela signifie que vous pouvez utiliser, modifier et distribuer ce projet, y compris à des fins commerciales, à condition de conserver la mention de copyright.

---