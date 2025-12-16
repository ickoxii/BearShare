/* app.js
 *
 * - Connects to your Axum websocket
 * - Sends ClientMessage JSON shaped like Rust serde(tag="type")
 * - Keeps a local JS RGA and converts textarea edits into Insert/Delete/Update ops
 */

(function () {
  "use strict";

  // ---------- UI helpers ----------
  const $ = (id) => document.getElementById(id);

  function log(msg) {
    const el = $("log");
    el.textContent += msg + "\n";
    el.scrollTop = el.scrollHeight;
  }

  function setWsStatus(s) { $("wsStatus").textContent = s; }
  function setRoomStatus(s) { $("roomStatus").textContent = s; }
  function setSiteStatus(s) { $("siteStatus").textContent = s; }

  function safeJsonParse(s) {
    try { return JSON.parse(s); } catch { return null; }
  }

  // ---------- WebSocket + state ----------
  let ws = null;

  let currentRoomId = null;
  let siteId = null;
  let numSites = null;

  /** Our local CRDT state. Only valid after JoinRoom/RoomCreated+Sync. */
  let rga = null;

  /** Prevent textarea "input" handler from generating ops when we apply remote changes. */
  let applyingRemote = false;

  function canSend() {
    return ws && ws.readyState === WebSocket.OPEN;
  }

  function sendClientMessage(obj) {
    if (!canSend()) {
      log("[send] not connected");
      return;
    }
    ws.send(JSON.stringify(obj));
  }

  // ---------- Protocol senders ----------
  function sendCreateRoom() {
    sendClientMessage({
      type: "CreateRoom",
      room_name: $("createRoomName").value.trim(),
      password: $("createPassword").value,
      filename: $("createFilename").value.trim(),
      initial_content: $("createInitial").value,
    });
  }

  function sendJoinRoom() {
    sendClientMessage({
      type: "JoinRoom",
      room_id: $("joinRoomId").value.trim(),
      password: $("joinPassword").value,
    });
  }

  function sendLeaveRoom() {
    sendClientMessage({ type: "LeaveRoom" });
  }

  function sendRequestSync() {
    sendClientMessage({ type: "RequestSync" });
  }

  function sendOperation(opEnvelope) {
    sendClientMessage({ type: "Operation", op: opEnvelope });
  }

  // ---------- Bootstrapping local RGA ----------
  function initRgaAndLoadText(baseText, bufferedOps) {
    if (siteId == null || numSites == null) {
      log("[rga] cannot init yet (missing siteId/numSites)");
      return;
    }

    rga = new window.Rga(siteId, numSites);

    // Build base content using deterministic server-site(0) inserts.
    rga.bootstrapFromPlainText(baseText);

    // Apply any ops since base content.
    if (Array.isArray(bufferedOps)) {
      for (const op of bufferedOps) {
        rga.applyRemote(op);
      }
    }

    applyingRemote = true;
    $("editor").value = rga.getText();
    applyingRemote = false;

    $("editor").disabled = false;
  }

  // ---------- Textarea diff -> ops ----------
  function diffStrings(oldStr, newStr) {
    const a = Array.from(oldStr);
    const b = Array.from(newStr);

    let start = 0;
    while (start < a.length && start < b.length && a[start] === b[start]) start++;

    let endA = a.length - 1;
    let endB = b.length - 1;
    while (endA >= start && endB >= start && a[endA] === b[endB]) {
      endA--;
      endB--;
    }

    return {
      start,
      oldMid: a.slice(start, endA + 1),
      newMid: b.slice(start, endB + 1),
    };
  }

  function onEditorInput() {
    if (applyingRemote) return;
    if (!rga) return;

    const oldText = rga.getText();
    const newText = $("editor").value;

    if (oldText === newText) return;

    const { start, oldMid, newMid } = diffStrings(oldText, newText);

    try {
      // Case A: pure insertion
      if (oldMid.length === 0 && newMid.length > 0) {
        let idx = start;
        for (const ch of newMid) {
          const op = rga.insertLocal(idx, ch);
          sendOperation(op);
          idx++;
        }
      }
      // Case B: pure deletion
      else if (newMid.length === 0 && oldMid.length > 0) {
        // Always delete at the same visible index start, because the text collapses left.
        for (let i = 0; i < oldMid.length; i++) {
          const op = rga.deleteLocal(start);
          sendOperation(op);
        }
      }
      // Case C: replacement (same length => Updates, else Delete+Insert)
      else {
        if (oldMid.length === newMid.length) {
          for (let i = 0; i < newMid.length; i++) {
            if (oldMid[i] !== newMid[i]) {
              const op = rga.updateLocal(start + i, newMid[i]);
              if (op) sendOperation(op);
            }
          }
        } else {
          for (let i = 0; i < oldMid.length; i++) {
            const op = rga.deleteLocal(start);
            sendOperation(op);
          }
          let idx = start;
          for (const ch of newMid) {
            const op = rga.insertLocal(idx, ch);
            sendOperation(op);
            idx++;
          }
        }
      }

      // Ensure textarea matches CRDT text after our local ops
      applyingRemote = true;
      $("editor").value = rga.getText();
      applyingRemote = false;
    } catch (e) {
      log("[editor] error: " + e.message);
      // Try to recover by requesting sync
      sendRequestSync();
    }
  }

  // ---------- Incoming server messages ----------
  function handleServerMessage(msg) {
    const t = msg.type;

    if (t === "RoomCreated") {
      currentRoomId = msg.room_id;
      siteId = msg.site_id;
      numSites = msg.num_sites;

      setRoomStatus(currentRoomId);
      setSiteStatus(`${siteId} / ${numSites}`);

      log(`[server] RoomCreated room_id=${currentRoomId} site_id=${siteId}`);
      // Auto-sync so we can load initial content into editor
      sendRequestSync();
    }
    else if (t === "JoinedRoom") {
      currentRoomId = msg.room_id;
      siteId = msg.site_id;
      numSites = msg.num_sites;

      setRoomStatus(currentRoomId);
      setSiteStatus(`${siteId} / ${numSites}`);

      log(`[server] JoinedRoom room_id=${currentRoomId} site_id=${siteId}`);
      initRgaAndLoadText(msg.document_content || "", msg.buffered_ops || []);
    }
    else if (t === "SyncResponse") {
      log("[server] SyncResponse");
      initRgaAndLoadText(msg.document_content || "", msg.buffered_ops || []);
    }
    else if (t === "Operation") {
      // from_site is provided by server
      if (!rga) return;

      try {
        rga.applyRemote(msg.op);
        applyingRemote = true;
        $("editor").value = rga.getText();
        applyingRemote = false;
      } catch (e) {
        log("[remote op] apply failed: " + e.message);
      }
    }
    else if (t === "Checkpoint") {
      // You can ignore this for basic UI; it's useful for debugging.
      log(`[server] Checkpoint ops_applied=${msg.ops_applied}`);
    }
    else if (t === "Error") {
      log("[server] ERROR: " + msg.message);
    }
    else if (t === "Pong") {
      log("[server] Pong");
    }
    else if (t === "UserJoined") {
      log(`[server] UserJoined site_id=${msg.site_id} user_id=${msg.user_id}`);
    }
    else if (t === "UserLeft") {
      log(`[server] UserLeft site_id=${msg.site_id} user_id=${msg.user_id}`);
    }
    else {
      log("[server] unknown msg: " + JSON.stringify(msg));
    }
  }

  // ---------- Connect / disconnect ----------
  function connect() {
    const url = $("wsUrl").value.trim();
    if (!url) return;

    ws = new WebSocket(url);
    setWsStatus("connecting...");

    ws.onopen = () => {
      setWsStatus("connected");
      log("[ws] open");
    };

    ws.onclose = () => {
      setWsStatus("disconnected");
      log("[ws] closed");
      ws = null;

      // local UI state
      currentRoomId = null;
      siteId = null;
      numSites = null;
      rga = null;

      setRoomStatus("none");
      setSiteStatus("n/a");

      $("editor").disabled = true;
      $("editor").value = "";
    };

    ws.onerror = () => {
      log("[ws] error");
    };

    ws.onmessage = (ev) => {
      const msg = safeJsonParse(ev.data);
      if (!msg) {
        log("[ws] non-json: " + ev.data);
        return;
      }
      handleServerMessage(msg);
    };
  }

  function disconnect() {
    if (ws) ws.close();
  }

  // ---------- wire up UI ----------
  $("btnConnect").addEventListener("click", connect);
  $("btnDisconnect").addEventListener("click", disconnect);

  $("btnCreateRoom").addEventListener("click", sendCreateRoom);
  $("btnJoinRoom").addEventListener("click", sendJoinRoom);
  $("btnLeaveRoom").addEventListener("click", sendLeaveRoom);
  $("btnSync").addEventListener("click", sendRequestSync);

  $("btnClearLog").addEventListener("click", () => { $("log").textContent = ""; });

  $("editor").addEventListener("input", onEditorInput);

  // initial UI state
  setWsStatus("disconnected");
  setRoomStatus("none");
  setSiteStatus("n/a");
})();
