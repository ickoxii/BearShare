/* rga.js
 *
 * A direct, readable port of the Rust RGA implementation:
 * - s4vector.rs (S4Vector + precedes)
 * - remote_op.rs (RemoteOp enum)
 * - rga.rs (insert/delete/update local + remote)
 *
 * Notes:
 * - We key nodes by S4Vector string "ssn|sid|sum|seq"
 * - "obj === null" means tombstone (deleted)
 */

(function (global) {
  "use strict";

  function s4Key(s4v) {
    return `${s4v.ssn}|${s4v.sid}|${s4v.sum}|${s4v.seq}`;
  }

  // Same ordering as Rust: (ssn, sum, sid)
  function s4Precedes(a, b) {
    if (a.ssn !== b.ssn) return a.ssn < b.ssn;
    if (a.sum !== b.sum) return a.sum < b.sum;
    return a.sid < b.sid;
  }

  // Shallow clone helpers to avoid accidental aliasing
  function cloneS4(s4v) {
    return { ssn: s4v.ssn, sid: s4v.sid, sum: s4v.sum, seq: s4v.seq };
  }

  function cloneVc(vc) {
    return vc.slice();
  }

  class Rga {
    /**
     * @param {number} siteId - site_id assigned by the server
     * @param {number} numSites - total sites (vector clock length)
     */
    constructor(siteId, numSites) {
      this.session = 1;
      this.siteId = siteId;
      this.vectorClock = new Array(numSites).fill(0);

      this.head = null;          // first node in linked list
      this.nodes = new Map();    // Map<s4Key, node>
      this.cemetery = [];        // list of deleted node ids (not required for basic editing)
    }

    /** Clears the structure but keeps siteId/numSites. */
    reset() {
      this.head = null;
      this.nodes.clear();
      this.cemetery = [];
      this.vectorClock.fill(0);
    }

    /** Returns the current visible text (tombstones omitted). */
    getText() {
      const out = [];
      let cur = this.head;
      while (cur) {
        if (cur.obj !== null) out.push(cur.obj);
        cur = cur.next;
      }
      return out.join("");
    }

    /** Returns array of nodes that are NOT tombstones, in visible order. */
    visibleNodes() {
      const out = [];
      let cur = this.head;
      while (cur) {
        if (cur.obj !== null) out.push(cur);
        cur = cur.next;
      }
      return out;
    }

    /** Finds the node at a given visible index (0-based). Returns null if out of range. */
    nodeAtVisibleIndex(index) {
      if (index < 0) return null;
      let cur = this.head;
      let i = 0;
      while (cur) {
        if (cur.obj !== null) {
          if (i === index) return cur;
          i++;
        }
        cur = cur.next;
      }
      return null;
    }

    /** Generates a new S4Vector for a LOCAL op (matches Rust generate_s4vector). */
    generateS4Vector() {
      this.vectorClock[this.siteId] += 1;

      let sum = 0;
      for (const v of this.vectorClock) sum += v;

      return {
        ssn: this.session,
        sid: this.siteId,
        sum: sum,
        seq: this.vectorClock[this.siteId],
      };
    }

    /** Updates our vector clock by taking elementwise max with a remote vector_clock. */
    mergeVectorClock(remoteVc) {
      const n = Math.min(this.vectorClock.length, remoteVc.length);
      for (let i = 0; i < n; i++) {
        this.vectorClock[i] = Math.max(this.vectorClock[i], remoteVc[i]);
      }
    }

    /** Inserts a node into the linked list after "ref". */
    linkAfter(ref, node) {
      node.next = ref.next;
      ref.next = node;
    }

    /** Remote insert (matches Rust insert_remote). */
    insertRemote(leftId, value, s4v) {
      const node = {
        obj: value,     // string length 1
        s_k: cloneS4(s4v),
        s_p: cloneS4(s4v),
        next: null,
      };
      this.nodes.set(s4Key(node.s_k), node);

      // Case 1: Insert at head (left_id == null)
      if (leftId == null) {
        if (this.head == null) {
          this.head = node;
          return;
        }

        // If current head precedes new s4v, put new before head.
        // (This matches Rust: if head.s_k.precedes(&s4v) => insert at head)
        if (s4Precedes(this.head.s_k, s4v)) {
          node.next = this.head;
          this.head = node;
          return;
        }

        // Otherwise scan starting at head to find insertion point.
        let ref = this.head;
        while (ref.next && s4Precedes(s4v, ref.next.s_k)) {
          ref = ref.next;
        }
        this.linkAfter(ref, node);
        return;
      }

      // Case 2: Insert after a specific left node
      const leftNode = this.nodes.get(s4Key(leftId));
      if (!leftNode) {
        // If you hit this: your client/server do not agree on IDs for "left_id".
        throw new Error("insertRemote: left_id not found in local RGA");
      }

      let ref = leftNode;
      while (ref.next && s4Precedes(s4v, ref.next.s_k)) {
        ref = ref.next;
      }
      this.linkAfter(ref, node);
    }

    /** Remote delete (matches Rust delete_remote). */
    deleteRemote(targetId, s4v) {
      const target = this.nodes.get(s4Key(targetId));
      if (!target) throw new Error("deleteRemote: target_id not found in local RGA");

      if (target.obj !== null) this.cemetery.push(cloneS4(target.s_k));
      target.obj = null;
      target.s_p = cloneS4(s4v);
    }

    /** Remote update (matches Rust update_remote). */
    updateRemote(targetId, value, s4v) {
      const target = this.nodes.get(s4Key(targetId));
      if (!target) throw new Error("updateRemote: target_id not found in local RGA");
      if (target.obj === null) return; // tombstone: ignore updates

      // Only update if existing s_p precedes new s4v
      if (s4Precedes(target.s_p, s4v)) {
        target.obj = value;
        target.s_p = cloneS4(s4v);
      }
    }

    /**
     * Applies a remote RemoteOp envelope from the wire:
     *   { Insert: { left_id, value, s4v, vector_clock } }
     *   { Delete: { target_id, s4v, vector_clock } }
     *   { Update: { target_id, value, s4v, vector_clock } }
     */
    applyRemote(opEnvelope) {
      const kind = Object.keys(opEnvelope)[0];
      const p = opEnvelope[kind];

      // Merge vector clock first (matches Rust receive_remote_op)
      this.mergeVectorClock(p.vector_clock);

      if (kind === "Insert") {
        this.insertRemote(p.left_id, p.value, p.s4v);
      } else if (kind === "Delete") {
        this.deleteRemote(p.target_id, p.s4v);
      } else if (kind === "Update") {
        this.updateRemote(p.target_id, p.value, p.s4v);
      } else {
        throw new Error("Unknown RemoteOp kind: " + kind);
      }
    }

    /** Local insert at visible index. Returns a RemoteOp envelope to send to server. */
    insertLocal(index, valueChar) {
      const s4v = this.generateS4Vector();

      let leftId = null;
      if (index > 0) {
        const leftNode = this.nodeAtVisibleIndex(index - 1);
        if (!leftNode) throw new Error("insertLocal: index out of range");
        leftId = cloneS4(leftNode.s_k);
      }

      // Apply locally using the same remote algorithm (matches Rust insert_local)
      this.insertRemote(leftId, valueChar, s4v);

      return {
        Insert: {
          left_id: leftId,
          value: valueChar,
          s4v: cloneS4(s4v),
          vector_clock: cloneVc(this.vectorClock),
        },
      };
    }

    /** Local delete at visible index. Returns a RemoteOp envelope to send to server. */
    deleteLocal(index) {
      const target = this.nodeAtVisibleIndex(index);
      if (!target) throw new Error("deleteLocal: index out of range");

      const s4v = this.generateS4Vector();

      if (target.obj !== null) this.cemetery.push(cloneS4(target.s_k));
      target.obj = null;
      target.s_p = cloneS4(s4v);

      return {
        Delete: {
          target_id: cloneS4(target.s_k),
          s4v: cloneS4(s4v),
          vector_clock: cloneVc(this.vectorClock),
        },
      };
    }

    /** Local update (replace char) at visible index. Returns a RemoteOp envelope to send to server. */
    updateLocal(index, valueChar) {
      const target = this.nodeAtVisibleIndex(index);
      if (!target) throw new Error("updateLocal: index out of range");
      if (target.obj === null) return null;

      const s4v = this.generateS4Vector();
      target.obj = valueChar;
      target.s_p = cloneS4(s4v);

      return {
        Update: {
          target_id: cloneS4(target.s_k),
          value: valueChar,
          s4v: cloneS4(s4v),
          vector_clock: cloneVc(this.vectorClock),
        },
      };
    }

    /**
     * Bootstraps from plain text by simulating the server's initial inserts
     * (server site_id = 0, session = 1, seq=sum=1..N).
     *
     * IMPORTANT: This only works if the server's base content IDs match that scheme.
     */
    bootstrapFromPlainText(text) {
      this.reset();

      const chars = Array.from(text); // unicode code points
      let left = null;

      for (let i = 0; i < chars.length; i++) {
        const vc = new Array(this.vectorClock.length).fill(0);
        vc[0] = i + 1;

        const s4v = { ssn: 1, sid: 0, sum: i + 1, seq: i + 1 };

        this.applyRemote({
          Insert: {
            left_id: left,
            value: chars[i],
            s4v: s4v,
            vector_clock: vc,
          },
        });

        left = cloneS4(s4v);
      }
    }
  }

  global.Rga = Rga;
})(window);
