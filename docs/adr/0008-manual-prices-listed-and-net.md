---
status: accepted
date: 2026-11-20
---

# Prices entered manually per channel: listed price and net price

## Context and Problem Statement

Owners want full control of prices on the site, Airbnb and Booking.com, and want to compare what each platform really earns them after commission.

## Decision Drivers

* No automatic price computation imposed on the owners
* Clear comparison between channels

## Considered Options

* Two manual fields per date range and per channel: **listed** (what the guest pays) and **net** (what the owners keep)
* One base price + automatic markup rules per channel

## Decision Outcome

Chosen option: "Two manual fields per range and per channel".

* A default price applies where no range is set (seed: net 100 € per night; editable).
* Commission settings per channel (commission %, payment fee %, VAT on commission) are used **only** to suggest the other field and to warn when values look inconsistent; suggestions are never applied automatically.
* A date open without a price for a channel is not bookable on that channel.

### Consequences

* Good, because owners stay in control and see real margins
* Bad, because more fields to fill (mitigated by copy tools: copy a year, copy a channel)

### Confirmation

Domain test: the price of a night is exactly the stored values, never computed.
