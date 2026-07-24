# RSTUDIO

## Objectif

Créer une station audio numérique (DAW) moderne en Rust.

## Règles du projet

- Utiliser du Rust idiomatique.
- Garder une architecture modulaire.
- Préférer des solutions simples et maintenables.
- Documenter les fonctions publiques.
- Éviter les dépendances inutiles.

## Contraintes importantes

- Le moteur audio doit être conçu pour une faible latence.
- Éviter les allocations mémoire dans le thread audio.
- Ne pas utiliser unwrap() dans le code de production.
- Toujours vérifier les erreurs proprement.

## Architecture actuelle

- app : application principale.
- common : types et utilitaires partagés.
- audio-engine : moteur audio temps réel.
- dsp : traitement numérique du signal.