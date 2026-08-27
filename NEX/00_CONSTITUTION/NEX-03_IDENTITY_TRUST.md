# NEX-03: Self-Sovereign Identity & Trust

## 1. Actor IDs
An `ActorID` is the cryptographic derivation of a public key:
`ActorID = SHA256("NEX/ACTOR_ID/v1" || KeyType || PublicKeyBytes)`

## 2. Web of Trust & Petnames
Global naming authorities (DNS, ICANN, centralized handles) are rejected. Names are local petnames resolved transitively through the user's Web of Trust with exponential score dampening: `Score = Score_A * Score_B * 0.5`.
