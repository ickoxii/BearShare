# BearShare

BearShare is a collaborative text editor that uses Replicated Growable Arrays (RGAs)[^1] to handle conflict-resolution and eventual consistency.

## Running the Project

Dependencies: `docker`, `npm`

<details>
  <summary>Full Containerization</summary>

  1. Build everything

     ```bash
     ./build.sh
     ```

     Navigate to `http://localhost:3000`

  1. Cleaning up

     ```bash
     ./teardown.sh
     ```

</details>

## Replicated Growable Arrays

How do we enforce conflict resolution when two collaborators are working on the same file? Traditional approaches have certain drawbacks. Locking is inefficient because only one collaborator can push edits to the file at a time. Operational transform can hurt interactivity. Take, for instance, Google Docs. Google Docs requires connection to Google's server. Even if you and your teammate are working in the same room, updates can take hundreds of milliseconds to propagate from one collaborator to another due to messages having to go from one user, to the server --- where operations must be applied server-side ---, then back to the other user.

The authors of "Replicated abstract data types"[^1] propose a family of Replicated Abstract Data Types (RADTs) to help combat these problems. These RADTs use the principle of operation commutativity and precedence transitivity to enforce eventual consistency. Each collaborator maintains a copy of the shared file, called a replica. Operations may be applied to replicas in any order. No matter what order operations are applied in, all replicas will eventually converge to the same state.

These concepts allow for more user flexibility. File editing no longer relies on a central server to apply updates and to be the single source of truth. RADTs inherently supports peer-to-peer connection and offline editing.

## References

[^1]: [Replicated abstract data types: Building blocks for collaborative applications (Roh et al., 2011)](http://csl.snu.ac.kr/papers/jpdc11.pdf)
