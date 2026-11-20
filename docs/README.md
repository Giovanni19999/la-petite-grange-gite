# Documentation

| Folder | Standard | Content |
|---|---|---|
| [architecture/](architecture/README.md) | arc42 (lite) + C4 diagrams in Mermaid | goals, constraints, context, containers, building blocks, deployment, cross-cutting concepts, risks |
| [adr/](adr/) | MADR 4 | architecture decision records; an accepted ADR is never edited, only superseded |
| [features/](features/README.md) | user story + Gherkin acceptance criteria | one sheet per feature, status `draft` / `frozen` / `implemented` |
| `api/openapi.json` | OpenAPI 3.1 | generated from the code (`utoipa`), added with the first business endpoints |
| [../CHANGELOG.md](../CHANGELOG.md) | Keep a Changelog + SemVer | release history |

Process: a feature is frozen before it is coded; any change goes through a new feature version or a new ADR. This folder contains no secrets and no personal data.
