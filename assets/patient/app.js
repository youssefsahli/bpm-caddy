// La page du patient : la messagerie de son officine, chiffrée de bout en
// bout avec WebCrypto. Le protocole est celui de src/patientchat.rs ; les
// valeurs de KAT y sont vérifiées par un test, et ici avant tout envoi.
'use strict';
// bpm-page: hors passerelle

const KAT = {
  client: 'e2ae25f709a44e263610be14252b440e71ddcfbcfdfbea76aecaf8be5688d788',
  server: 'c4441c7f787363f4e263bee9b3043830e6328e4f032671d2a3e27e199e330167',
  closed: '4620447ef9ec84d374c61162b26bf1ee13dff689608b6eed5ee639fa21cf006c',
  sealed: '7569bf2885d9a3b1d898da2a820ca83b350bbdbf2c7d9f',
};
const enc = new TextEncoder();
const dec = new TextDecoder();
const SALT = enc.encode('bpm-caddy-patient-1');
const MAGIC = enc.encode('BPMP1');
const CLOSE_ABSENT = 4004;
const STORE = 'bpm-caddy-messagerie';
const POLL_MS = 30000;
const TALK_MS = 30000;

const $ = (id) => document.getElementById(id);

function hex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function cat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

function fromB64url(text) {
  const b = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(b, (c) => c.charCodeAt(0));
}

function random(n) {
  return crypto.getRandomValues(new Uint8Array(n));
}

async function hkdf(salt, ikm, info) {
  const k = await crypto.subtle.importKey('raw', ikm, 'HKDF', false, ['deriveBits']);
  const bits = await crypto.subtle.deriveBits(
    { name: 'HKDF', hash: 'SHA-256', salt, info: enc.encode(info) }, k, 256);
  return new Uint8Array(bits);
}

async function hmac(key, ...parts) {
  const k = await crypto.subtle.importKey(
    'raw', key, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return new Uint8Array(await crypto.subtle.sign('HMAC', k, cat(...parts)));
}

const authKey = (k) => hkdf(SALT, k, 'auth');

function nonce(n) {
  const iv = new Uint8Array(12);
  new DataView(iv.buffer).setBigUint64(4, BigInt(n));
  return iv;
}

async function sealer(raw) {
  const key = await crypto.subtle.importKey('raw', raw, 'AES-GCM', false, ['encrypt', 'decrypt']);
  let n = 0;
  return {
    seal: async (plain) => new Uint8Array(
      await crypto.subtle.encrypt({ name: 'AES-GCM', iv: nonce(n++) }, key, plain)),
    open: async (sealed) => new Uint8Array(
      await crypto.subtle.decrypt({ name: 'AES-GCM', iv: nonce(n++) }, key, sealed)),
  };
}

async function session(k, nc, ns) {
  const salt = cat(nc, ns);
  return {
    up: await sealer(await hkdf(salt, k, 'patient>officine')),
    down: await sealer(await hkdf(salt, k, 'officine>patient')),
  };
}

// The browser seals as src/patientchat.rs does, or it says nothing.
async function selfTest() {
  try {
    const k = new Uint8Array(32).fill(0x11);
    const nc = new Uint8Array(16).fill(0x22);
    const ns = new Uint8Array(16).fill(0x33);
    const a = await authKey(k);
    if (hex(await hmac(a, enc.encode('client'), nc)) !== KAT.client) return false;
    if (hex(await hmac(a, enc.encode('server'), nc, ns)) !== KAT.server) return false;
    if (hex(await hmac(a, enc.encode('ferme'), nc)) !== KAT.closed) return false;
    const s = await session(k, nc, ns);
    return hex(await s.up.seal(enc.encode('bonjour'))) === KAT.sealed;
  } catch (e) {
    return false;
  }
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  let d = 0;
  for (let i = 0; i < a.length; i++) d |= a[i] ^ b[i];
  return d === 0;
}

// --- The ways to the officine ---------------------------------------------

const BOARD = enc.encode('BPMT1');
const CHUNK = 16384;
const DIRECT_MS = 12000;

function b64urlToHex(text) {
  return Array.from(fromB64url(text), (b) => b.toString(16).padStart(2, '0').toUpperCase()).join(':');
}

// "203.0.113.7:7748" or "[2001:db8::1]:7748" -> [ip, port, v6].
function hostPort(a) {
  const m = /^\[([0-9a-fA-F:.]+)\]:(\d{1,5})$/.exec(a) || /^([0-9.]+):(\d{1,5})$/.exec(a);
  return m ? [m[1], m[2], m[1].includes(':')] : null;
}

// **A data channel's bytes**, once open: four bytes of length, then the
// frame, cut into messages every browser accepts. `ok` gets the stream,
// `ko` why it never opened (`absent` after `ms`).
function channel(pc, dc, ms, ok, ko) {
  dc.binaryType = 'arraybuffer';
  let buf = new Uint8Array(0);
  const frames = [];
  let waiter = null;
  let opened = false;
  const fail = (e) => {
    if (waiter) { const w = waiter; waiter = null; w.ko(e); }
  };
  const give = (e) => {
    clearTimeout(timer);
    try { pc.close(); } catch (x) { /* closed */ }
    ko(e);
  };
  const timer = setTimeout(() => { if (!opened) give(new Error('absent')); }, ms);
  dc.onmessage = (ev) => {
    buf = cat(buf, new Uint8Array(ev.data));
    while (buf.length >= 4) {
      const n = new DataView(buf.buffer, buf.byteOffset).getUint32(0);
      if (n > 262144) { fail(new Error('trame')); return; }
      if (buf.length < 4 + n) break;
      const f = buf.slice(4, 4 + n);
      buf = buf.slice(4 + n);
      if (waiter) { const w = waiter; waiter = null; w.ok(f); } else frames.push(f);
    }
  };
  dc.onclose = () => fail(new Error('fermée'));
  dc.onopen = () => {
    opened = true;
    clearTimeout(timer);
    ok({
      send(frame) {
        const len = new Uint8Array(4);
        new DataView(len.buffer).setUint32(0, frame.length);
        const all = cat(len, frame);
        for (let i = 0; i < all.length; i += CHUNK) dc.send(all.slice(i, i + CHUNK));
      },
      next: () => new Promise((yes, no) => {
        if (frames.length) yes(frames.shift());
        else waiter = { ok: yes, ko: no };
      }),
      close() { try { pc.close(); } catch (e) { /* closed */ } },
    });
  };
  pc.onconnectionstatechange = () => {
    if (pc.connectionState === 'failed' && !opened) give(new Error('absent'));
  };
  return give;
}

// **The direct way** (src/direct.rs): a WebRTC data channel to the
// officine's posts, with no signalling server. The link carries what a
// server would have passed: the posts' certificate fingerprint and their
// addresses; the ICE credentials come from the box. The page writes the
// posts' answer itself — every address a candidate, the first that
// answers kept.
function openDirect(state) {
  return new Promise((resolve, reject) => {
    const where = (state.a || []).map(hostPort).filter(Boolean);
    if (!state.f || !where.length || !window.RTCPeerConnection) {
      reject(new Error('pas de voie directe'));
      return;
    }
    const pc = new RTCPeerConnection();
    const give = channel(pc, pc.createDataChannel('bpm'), DIRECT_MS, resolve, reject);
    (async () => {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      const mid = (/a=mid:(\S+)/.exec(offer.sdp) || [, '0'])[1];
      const lines = [
        'v=0', 'o=- 1 1 IN IP4 0.0.0.0', 's=-', 't=0 0', 'a=ice-lite',
        `a=group:BUNDLE ${mid}`,
        'm=application 9 UDP/DTLS/SCTP webrtc-datachannel',
        'c=IN IP4 0.0.0.0',
        `a=mid:${mid}`,
        `a=ice-ufrag:${state.b.slice(0, 8)}`,
        `a=ice-pwd:${state.b}`,
        `a=fingerprint:sha-256 ${b64urlToHex(state.f)}`,
        'a=setup:passive',
        'a=sctp-port:5000',
        'a=max-message-size:262144',
      ];
      where.forEach(([ip, port], i) => {
        lines.push(`a=candidate:${i + 1} 1 udp ${2130706431 - i} ${ip} ${port} typ host`);
      });
      lines.push('a=end-of-candidates', '');
      await pc.setRemoteDescription({ type: 'answer', sdp: lines.join('\r\n') });
    })().catch(give);
  });
}

// **The meeting** (src/rendezvous.rs): when nothing reaches the posts —
// no open box, no public IPv6 — both sides go out. The page asks public
// STUN servers what its box gives it, and publishes its offer in the
// officine's swarm at public WebTorrent trackers, which hand it to the
// posts waiting there and bring their answer back; then both send at
// once and the channel is direct. The trackers see addresses, never a
// message; an answer counts only with the officine's fingerprint, which
// DTLS then holds the post to.
const TRACKERS = ['wss://tracker.openwebtorrent.com', 'wss://tracker.webtorrent.dev'];
const STUN = ['stun:stun.l.google.com:19302', 'stun:stun.cloudflare.com:3478'];
const MEET_MS = 20000;

async function swarmOf(b) {
  const d = await crypto.subtle.digest('SHA-256', enc.encode(`bpm-caddy rendez-vous 1|${b}`));
  return hex(new Uint8Array(d).slice(0, 10));
}

// Twenty characters, as a tracker wants a peer or an offer named.
function drawId() {
  return hex(random(10));
}

// The offer once the browser has found its addresses (or after a while:
// what it has found by then).
function gathered(pc, ms) {
  return new Promise((ok) => {
    if (pc.iceGatheringState === 'complete') { ok(); return; }
    const t = setTimeout(ok, ms);
    pc.addEventListener('icegatheringstatechange', () => {
      if (pc.iceGatheringState === 'complete') { clearTimeout(t); ok(); }
    });
  });
}

function openMet(state) {
  return new Promise((resolve, reject) => {
    if (!state.f || !state.r || !window.RTCPeerConnection || !window.WebSocket) {
      reject(new Error('pas de rendez-vous'));
      return;
    }
    const pc = new RTCPeerConnection({ iceServers: STUN.map((urls) => ({ urls })) });
    const sockets = [];
    const hangUp = () => sockets.forEach((ws) => { try { ws.close(); } catch (e) { /* closed */ } });
    const give = channel(pc, pc.createDataChannel('bpm'), MEET_MS,
      (t) => { hangUp(); resolve(t); },
      (e) => { hangUp(); reject(e); });
    (async () => {
      await pc.setLocalDescription(await pc.createOffer());
      await gathered(pc, 3000);
      const sdp = pc.localDescription.sdp;
      const swarm = await swarmOf(state.b);
      const me = drawId();
      const offerId = drawId();
      const want = b64urlToHex(state.f);
      let answered = false;
      for (const url of TRACKERS) {
        let ws;
        try { ws = new WebSocket(url); } catch (e) { continue; }
        sockets.push(ws);
        ws.onerror = () => {};
        ws.onopen = () => ws.send(JSON.stringify({
          action: 'announce', info_hash: swarm, peer_id: me, numwant: 1,
          uploaded: 0, downloaded: 0, left: 1, event: 'started',
          offers: [{ offer_id: offerId, offer: { type: 'offer', sdp } }],
        }));
        ws.onmessage = async (ev) => {
          let m;
          try { m = JSON.parse(ev.data); } catch (e) { return; }
          if (answered || m.info_hash !== swarm || m.offer_id !== offerId) return;
          if (!m.answer || m.answer.type !== 'answer' || typeof m.answer.sdp !== 'string') return;
          const fp = /a=fingerprint:sha-256 ([0-9A-Fa-f:]+)/.exec(m.answer.sdp);
          if (!fp || fp[1].toUpperCase() !== want) return;
          answered = true;
          try {
            await pc.setRemoteDescription({ type: 'answer', sdp: m.answer.sdp });
          } catch (e) {
            answered = false;
          }
        };
      }
    })().catch(give);
  });
}

// The direct way and the meeting at once: the first channel open is
// kept, the other closed when it opens too.
function openNear(state) {
  const tries = [openDirect, openMet].map((open) => open(state));
  return new Promise((resolve, reject) => {
    let left = tries.length;
    let won = false;
    let absent = false;
    tries.forEach((p) => p.then((t) => {
      if (won) { t.close(); return; }
      won = true;
      resolve(t);
    }, (e) => {
      absent = absent || e.message === 'absent';
      left -= 1;
      if (!left && !won) reject(new Error(absent ? 'absent' : 'injoignable'));
    }));
  });
}

// **A gateway** (src/passerelle.rs), when the officine has one: one
// WebSocket message per frame.
function openGateway(state) {
  return new Promise((resolve, reject) => {
    if (!state.p) { reject(new Error('pas de passerelle')); return; }
    // Plain ws only for a gateway on this machine (tests).
    const scheme = /^127\.0\.0\.1:/.test(state.p) ? 'ws:' : 'wss:';
    const ws = new WebSocket(`${scheme}//${state.p}/ws/${state.b}`);
    ws.binaryType = 'arraybuffer';
    const frames = [];
    let waiter = null;
    let opened = false;
    ws.onmessage = (ev) => {
      const m = new Uint8Array(ev.data);
      if (waiter) { const w = waiter; waiter = null; w.ok(m); } else frames.push(m);
    };
    ws.onclose = (ev) => {
      const e = new Error(ev.code === CLOSE_ABSENT ? 'absent' : 'fermée');
      if (!opened) reject(e);
      else if (waiter) { const w = waiter; waiter = null; w.ko(e); }
    };
    ws.onerror = () => {};
    ws.onopen = () => {
      opened = true;
      resolve({
        send(frame) { ws.send(frame); },
        next: () => new Promise((ok, ko) => {
          if (frames.length) ok(frames.shift());
          else waiter = { ok, ko };
        }),
        close() { try { ws.close(); } catch (e) { /* closed */ } },
      });
    };
  });
}

// The direct way (or the meeting) first, then the gateway. Nobody on
// either: absent. `ways` narrows it: the board never comes through a
// gateway — nothing there proves it came from the officine; a channel
// held to the officine's fingerprint does.
async function reach(state, ways = [openNear, openGateway]) {
  let absent = false;
  for (const open of ways) {
    try {
      return await open(state);
    } catch (e) {
      absent = absent || e.message === 'absent';
    }
  }
  throw new Error(absent ? 'absent' : 'injoignable');
}

function within(ms, promise) {
  return Promise.race([
    promise,
    new Promise((_, ko) => setTimeout(() => ko(new Error('délai')), ms)),
  ]);
}

// One conversation: hello, the post's proof, the request, the answer.
// Nobody waiting for the page (no post reached, or the gateway says so):
// absent, and the messages stay on the phone.
async function converse(state, request) {
  try {
    return await talk(state, request);
  } catch (e) {
    if (e.message === 'absent') return { absent: true };
    throw e;
  }
}

async function talk(state, request) {
  const t = await reach(state);
  try {
    return await within(TALK_MS, (async () => {
      const key = state.key;
      const nc = random(16);
      const a = await authKey(key);
      t.send(cat(MAGIC, nc, await hmac(a, enc.encode('client'), nc)));
      const back = await t.next();
      // « Closed » counts only signed by whoever holds this key: a bare
      // word from the gateway would shut the patient out for good.
      if (back.length === 37 && dec.decode(back.slice(0, 5)) === 'FERME') {
        if (!sameBytes(await hmac(a, enc.encode('ferme'), nc), back.slice(5))) {
          throw new Error('preuve');
        }
        return { closed: true };
      }
      if (dec.decode(back) === 'INCONNU') return { unknown: true };
      if (back.length !== 48) throw new Error('réponse');
      const ns = back.slice(0, 16);
      if (!sameBytes(await hmac(a, enc.encode('server'), nc, ns), back.slice(16))) {
        throw new Error('preuve');
      }
      const s = await session(key, nc, ns);
      t.send(await s.up.seal(enc.encode(JSON.stringify(request))));
      const answer = JSON.parse(dec.decode(await s.down.open(await t.next())));
      return { answer };
    })());
  } finally {
    t.close();
  }
}

// **The officine's board** (src/tableau.rs), for anyone: no key.
async function fetchBoard(state) {
  const t = await reach(state, [openNear]);
  try {
    return await within(TALK_MS, (async () => {
      t.send(BOARD);
      return JSON.parse(dec.decode(await t.next()));
    })());
  } finally {
    t.close();
  }
}

// --- What the phone keeps -------------------------------------------------

let state = null;
let storable = true;

// One conversation per officine (its box): every officine's page lives
// on the same site, and one patient may write to two of them.
const slot = (b) => `${STORE}:${b}`;

function load(b) {
  try {
    const raw = localStorage.getItem(slot(b));
    return raw ? JSON.parse(raw) : null;
  } catch (e) {
    storable = false;
    return null;
  }
}

// The officines this phone holds a conversation with.
function boxesKept() {
  try {
    return Object.keys(localStorage)
      .filter((k) => k.startsWith(`${STORE}:`))
      .map((k) => k.slice(STORE.length + 1));
  } catch (e) {
    return [];
  }
}

function save() {
  if (!state) return;
  try {
    const { key, ...kept } = state;
    localStorage.setItem(slot(state.b), JSON.stringify(kept));
  } catch (e) {
    storable = false;
  }
}

// The posts' addresses, as a link or a post gives them.
function addresses(list) {
  return (list || []).filter((a) => typeof a === 'string' && hostPort(a)).slice(0, 6);
}

function fromLink() {
  const p = new URLSearchParams(location.hash.slice(1));
  const b = (p.get('b') || '').toLowerCase();
  const k = p.get('k') || '';
  const f = p.get('f') || '';
  const a = addresses((p.get('a') || '').split(','));
  // A code of 0.433.0 named no gateway: that gateway sends it here with
  // its own name in the query.
  const g = (p.get('p') || new URLSearchParams(location.search).get('p') || '').toLowerCase();
  if (!/^[0-9a-f]{32}$/.test(b)) return null;
  const way = {
    f: /^[A-Za-z0-9_-]{43}$/.test(f) ? f : '',
    a,
    r: p.get('r') === '1',
    p: /^(\[[0-9a-f:]+\]|[a-z0-9.-]+)(:[0-9]{1,5})?$/.test(g) ? g : '',
  };
  const reachable = (way.f && (way.a.length || way.r)) || way.p;
  // The board's code: no key, the board only — on a phone, or on a
  // screen in the window (`e`).
  if (p.get('t') === '1') return reachable ? { b, board: true, screen: p.get('e') === '1', ...way } : null;
  if (!/^[A-Za-z0-9_-]{43}$/.test(k)) return { b };
  if (!reachable) return null;
  return { b, k, ...way };
}

function newId() {
  return hex(random(8));
}

function nowText() {
  const d = new Date();
  const z = (n) => String(n).padStart(2, '0');
  return `${z(d.getDate())}/${z(d.getMonth() + 1)}/${d.getFullYear()} ${z(d.getHours())}:${z(d.getMinutes())}`;
}

// --- The page ---------------------------------------------------------------

function say(text, error) {
  const e = $('etat');
  e.textContent = text || '';
  e.className = error ? 'erreur' : '';
}

const CANAUX = { garde: 'Pharmacie de garde', produit: 'Disponible ici', info: 'Informations' };

// "2026-10-16" -> "16/10".
function dayText(iso) {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso || '');
  return m ? `${m[3]}/${m[2]}` : '';
}

function whenText(l) {
  const du = dayText(l.du);
  const au = dayText(l.au);
  if (du && au) return du === au ? `le ${du}` : `du ${du} au ${au}`;
  if (au) return `jusqu'au ${au}`;
  if (du) return `à partir du ${du}`;
  return '';
}

// **The board**, channel by channel, in the order the post sent it.
// Nothing on it: no section at all.
function drawBoard(lines, into) {
  into.replaceChildren();
  const shown = (lines || []).filter((l) => l && CANAUX[l.canal] && l.titre);
  if (!shown.length) {
    into.hidden = true;
    return;
  }
  for (const canal of Object.keys(CANAUX)) {
    const mine = shown.filter((l) => l.canal === canal);
    if (!mine.length && canal !== 'garde') continue;
    const h = document.createElement('h2');
    h.textContent = CANAUX[canal];
    into.append(h);
    for (const l of mine) {
      const row = document.createElement('div');
      row.className = `rangee ${canal}`;
      const t = document.createElement('span');
      t.className = 'titre';
      t.textContent = l.titre;
      const w = document.createElement('span');
      w.className = 'quand';
      w.textContent = whenText(l);
      row.append(t, w);
      if (l.detail) {
        const d = document.createElement('span');
        d.className = 'detail';
        d.textContent = l.detail;
        row.append(d);
      }
      into.append(row);
    }
    // The national number for the pharmacy on duty, whatever the board says.
    if (canal === 'garde') {
      const p = document.createElement('p');
      p.className = 'discret';
      p.textContent = 'Pharmacie de garde la plus proche : 3237 (service Résogardes).';
      into.append(p);
    }
  }
  into.hidden = false;
}

function drawWho(who, name) {
  if (name) $('officine').textContent = name;
  $('rt').textContent = [name || 'votre pharmacie', who.adresse, who.telephone]
    .filter(Boolean).join(', ');
  $('horaires').textContent = who.horaires ? `Ouverte : ${who.horaires}` : '';
  $('horaires').hidden = !who.horaires;
}

function draw() {
  const fil = $('fil');
  fil.replaceChildren();
  drawWho(state.who || {}, state.officine);
  // The board as the post last gave it — and when: one kept on the phone
  // may be old.
  drawBoard(state.board || [], $('tableau'));
  $('lu').textContent = state.board && state.board.length && state.boardAt
    ? `Informations lues le ${state.boardAt}` : '';
  if (!state.messages.length) {
    const p = document.createElement('p');
    p.className = 'vide';
    p.textContent = 'Aucun message pour l\'instant.';
    fil.append(p);
  }
  for (const m of state.messages) {
    const div = document.createElement('div');
    div.className = `bulle ${m.mine ? 'moi' : 'eux'}`;
    div.textContent = m.text;
    const when = document.createElement('span');
    when.className = 'quand';
    when.textContent = m.mine ? `${m.at}${m.pending ? ' · en attente' : ' · reçu'}` : m.at;
    div.append(when);
    fil.append(div);
  }
  const left = 2000 - $('texte').value.length;
  $('compte').textContent = left < 200 ? `${left} caractères restants` : '';
}

let busy = false;

async function sync(quiet) {
  if (busy || !state || state.over) return;
  busy = true;
  const pending = state.messages.filter((m) => m.mine && m.pending);
  if (!quiet) say(pending.length ? 'Envoi…' : 'Mise à jour…');
  try {
    const r = await converse(state, {
      out: pending.slice(0, 20).map((m) => ({ id: m.id, at: m.at, text: m.text })),
      // By number, not « since »: each post draws its own.
      seen: state.messages.filter((m) => !m.mine).map((m) => m.id).slice(-500),
    });
    if (r.absent) {
      say(pending.length
        ? 'La pharmacie n\'est pas joignable pour l\'instant. Votre message reste sur ce téléphone et partira dès qu\'elle le sera : gardez cette page ouverte, ou revenez plus tard.'
        : 'La pharmacie n\'est pas joignable pour l\'instant.');
    } else if (r.closed) {
      state.over = true;
      say('La pharmacie a fermé cet accès. Demandez-lui un nouveau code si besoin.', true);
      $('ecrire').hidden = true;
    } else if (r.unknown) {
      // A code just handed over may not have reached the post that
      // answered yet: tried again, never given up on.
      say('Ce code n\'est pas encore reconnu par la pharmacie. Nouvel essai dans un moment ; s\'il ne l\'est toujours pas demain, demandez-lui un nouveau code.', true);
    } else {
      const a = r.answer;
      const got = new Set(a.got || []);
      for (const m of state.messages) if (m.mine && got.has(m.id)) m.pending = false;
      for (const m of a.in || []) {
        if (!state.messages.some((x) => !x.mine && x.id === m.id)) {
          state.messages.push({ id: m.id, at: m.at, text: m.text, mine: false });
        }
      }
      if (a.officine) state.officine = a.officine;
      state.who = { adresse: a.adresse || '', telephone: a.telephone || '', horaires: a.horaires || '' };
      // The posts' addresses as they are now: an officine whose address
      // changed stays reachable from whoever wrote to it since.
      const fresh = addresses(a.adresses);
      if (fresh.length) state.a = fresh;
      state.board = Array.isArray(a.tableau) ? a.tableau : [];
      state.boardAt = nowText();
      say(pending.length ? 'Message reçu par la pharmacie.' : '');
    }
  } catch (e) {
    if (!quiet || pending.length) say('La connexion a échoué. Nouvel essai dans un moment.', true);
  } finally {
    busy = false;
    save();
    draw();
  }
}

function send() {
  const t = $('texte');
  const text = t.value.trim();
  if (!text) return;
  state.messages.push({ id: newId(), at: nowText(), text: text.slice(0, 2000), mine: true, pending: true });
  t.value = '';
  save();
  draw();
  sync(false);
}

// **The board alone**: the officine's code in its window or on its site,
// no key and nothing kept. On a screen (`e`), large, read again every
// minute — a departures board for the waiting room or the window.
async function boardOnly(link) {
  document.body.classList.add('seul');
  if (link.screen) document.body.classList.add('ecran');
  $('ecrire').hidden = true;
  $('fil').hidden = true;
  $('sur').textContent = 'Informations';
  const view = { b: link.b, f: link.f, a: link.a, r: link.r };
  let shown = false;
  const read = async () => {
    try {
      const board = await fetchBoard(view);
      // A screen left on for weeks follows the posts' addresses.
      const fresh = addresses(board.adresses);
      if (fresh.length) view.a = addresses([...fresh, ...view.a.filter((x) => !fresh.includes(x))]);
      drawWho(board, board.officine || '');
      drawBoard(board.lignes, $('tableau'));
      $('lu').textContent = `Mis à jour à ${nowText().slice(-5)}`;
      shown = true;
      say('');
    } catch (e) {
      // A screen keeps what it showed, and says it is old.
      say(shown
        ? 'La pharmacie n\'est pas joignable : informations de la dernière mise à jour.'
        : 'La pharmacie n\'est pas joignable pour l\'instant.', !shown);
    }
  };
  await read();
  setInterval(() => {
    if (link.screen || document.visibilityState === 'visible') read();
  }, link.screen ? 60000 : POLL_MS);
}

async function start() {
  const link = fromLink();
  if (link && link.board) {
    boardOnly(link);
    return;
  }
  if (!window.crypto || !crypto.subtle || !(await selfTest())) {
    say('Ce navigateur ne permet pas le chiffrement de la messagerie. Essayez avec un navigateur à jour.', true);
    return;
  }
  // Which officine: the link's, or the only one this phone knows.
  const boxes = boxesKept();
  const b = link ? link.b : (boxes.length === 1 ? boxes[0] : '');
  const kept = b ? load(b) : null;
  if (link && link.k) {
    state = kept && kept.k === link.k
      ? kept
      : { b: link.b, k: link.k, messages: [], officine: '' };
    // The ways of the code just scanned first, then the addresses the
    // posts gave since: a new code is what mends a phone whose addresses
    // all went stale.
    state.p = link.p;
    state.f = link.f;
    state.r = link.r;
    state.a = addresses([...link.a, ...(state.a || []).filter((x) => !link.a.includes(x))]);
  } else if (kept && kept.b && kept.k && ((kept.f && ((kept.a && kept.a.length) || kept.r)) || kept.p)) {
    state = kept;
  } else {
    say('Ouvrez cette page en scannant le code remis par votre pharmacie.', true);
    return;
  }
  state.key = fromB64url(state.k);
  save();
  // The key out of the address bar once the phone keeps it: a screenshot
  // or a shared link does not carry it.
  // Only the box stays, which names the officine and not the patient: a
  // reload finds the conversation, a screenshot does not carry the key.
  if (link && storable) history.replaceState(null, '', `${location.pathname}#b=${state.b}`);
  $('ecrire').hidden = !!state.over;
  $('effacer').hidden = false;
  if (state.over) say('La pharmacie a fermé cet accès. Demandez-lui un nouveau code si besoin.', true);
  $('texte').addEventListener('input', draw);
  $('envoyer').addEventListener('click', send);
  $('effacer').addEventListener('click', () => {
    if (!confirm('Effacer cette conversation de ce téléphone ? Il faudra scanner de nouveau le code pour écrire à la pharmacie.')) return;
    try { localStorage.removeItem(slot(state.b)); } catch (e) { /* nothing kept */ }
    location.replace(location.pathname);
  });
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') sync(true);
  });
  setInterval(() => { if (document.visibilityState === 'visible') sync(true); }, POLL_MS);
  draw();
  sync(true);
}

start();
