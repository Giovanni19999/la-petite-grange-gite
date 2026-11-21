---
status: accepted
date: 2026-11-21
---

# Listed and net prices fill each other as they are typed

## Context and Problem Statement

ADR-0008 asks the owners to type two prices per channel (listed, net) and only *suggests* the other one, never filling it. Trying the screen, the owners asked that both fields be editable **and** computed from each other: typing one should fill the other, so that they do not do the commission calculation nor click a suggestion for every price. This supersedes the bullet of ADR-0008 "suggestions are never applied automatically"; the rest of ADR-0008 stays.

## Decision Drivers

* Fewer clicks for the owners, who set many prices
* The owners keep the last word on every stored price
* A stored price must never change by itself afterwards

## Considered Options

* Typing one field fills the other with the channel's rate; both stay editable
* Keep the greyed suggestion of ADR-0008 (one click to take it)

## Decision Outcome

Chosen option: "Typing one field fills the other".

* When the owner types the listed price, the net price is filled with `listed × (1 − t)`; when they type the net price, the listed price is filled with `net / (1 − t)`, rounded to the euro, with `t` the channel's effective rate (commission + payment fees, × 1.2 with VAT on the commission). For the site (`t = 0`), both are equal.
* The field typed last drives the other; the owner can then correct the computed field by hand.
* What is saved is exactly what the two fields show when the owner saves: the API stores the values it receives, it does not compute them.
* A later change of a channel's rate never recomputes prices already stored.
* The warning when the implicit commission differs from the channel's rate stays (useful after a manual correction).

### Consequences

* Good, because most prices need one value typed instead of two
* Good, because stored prices stay exactly what the owners saw and saved
* Bad, because an owner may save a computed value without looking at it (the warning and the typical stays help to check)

### Confirmation

Front test: typing one field fills the other with the rate of the channel; the API test checks that saved prices are the values sent; changing a rate leaves stored prices unchanged.
