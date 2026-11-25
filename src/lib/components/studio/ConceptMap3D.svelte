<script lang="ts">
  import { onMount } from 'svelte';
  import type * as ThreeNS from 'three';
  import type { SessionVisualization } from '$lib/api/aiStudy';
  import { layoutGraph3D } from '$lib/utils/graphLayout3d';
  import { colorForGroup, groupsOf } from '$lib/utils/conceptPalette';

  let {
    visualization,
    selectedId = null,
    activeIds = new Set<string>(),
    follow = false,
    reduceMotion = false,
    resetTick = 0,
    zoomTick = 0,
    zoomDir = 1,
    onSelect,
    onUnsupported,
    onReady,
  }: {
    visualization: SessionVisualization;
    selectedId?: string | null;
    /** Concepts being discussed at the playhead; they pulse. */
    activeIds?: Set<string>;
    /** Ease the camera toward the active concepts as the lecture plays. */
    follow?: boolean;
    reduceMotion?: boolean;
    /** Bump to re-fit the whole graph in view. */
    resetTick?: number;
    /** Bump to zoom by `zoomDir` (+1 in, -1 out). */
    zoomTick?: number;
    zoomDir?: 1 | -1;
    onSelect?: (id: string | null) => void;
    /** Called if WebGL cannot start, so the parent can show the 2D map. */
    onUnsupported?: () => void;
    onReady?: () => void;
  } = $props();

  const nodes = $derived(
    layoutGraph3D(
      visualization.spec.nodes ?? [],
      visualization.spec.edges ?? []
    )
  );
  const groups = $derived(groupsOf(nodes));
  const edges = $derived(
    (visualization.spec.edges ?? []).filter(
      (e) =>
        nodes.some((n) => n.id === e.from) && nodes.some((n) => n.id === e.to)
    )
  );

  let host: HTMLDivElement | undefined = $state();
  let canvas: HTMLCanvasElement | undefined = $state();
  let labelEls = $state<Record<string, HTMLElement | undefined>>({});
  let ready = $state(false);

  // The scene is imperative; these hold its live objects between effects.
  type Engine = {
    setSelected(id: string | null): void;
    setActive(ids: Set<string>): void;
    setFollow(on: boolean): void;
    setReduced(on: boolean): void;
    reset(): void;
    zoom(dir: 1 | -1): void;
    rebuild(key: string): void;
    dispose(): void;
  };
  // $state.raw so the prop-sync effects below re-run once the engine exists.
  let engine = $state.raw<Engine | undefined>();

  function webglAvailable(): boolean {
    try {
      const c = document.createElement('canvas');
      return !!(c.getContext('webgl2') || c.getContext('webgl'));
    } catch {
      return false;
    }
  }

  onMount(() => {
    let cancelled = false;
    if (!webglAvailable()) {
      onUnsupported?.();
      return;
    }
    (async () => {
      try {
        // Three.js is large; it only loads once someone opens the 3D map.
        const [THREE, { OrbitControls }] = await Promise.all([
          import('three'),
          import('three/examples/jsm/controls/OrbitControls.js'),
        ]);
        if (cancelled || !host || !canvas) return;
        engine = createEngine(THREE, OrbitControls);
        ready = true;
        onReady?.();
      } catch {
        if (!cancelled) onUnsupported?.();
      }
    })();
    return () => {
      cancelled = true;
      engine?.dispose();
      engine = undefined;
    };
  });

  // React to prop changes.
  $effect(() => engine?.setSelected(selectedId));
  $effect(() => engine?.setActive(activeIds));
  $effect(() => engine?.setFollow(follow));
  $effect(() => engine?.setReduced(reduceMotion));
  $effect(() => {
    void resetTick;
    engine?.reset();
  });
  $effect(() => {
    if (zoomTick > 0) engine?.zoom(zoomDir);
  });
  $effect(() => {
    // A different map (or regenerated graph) needs a fresh scene.
    engine?.rebuild(`${visualization.id}:${nodes.length}`);
  });

  function createEngine(
    THREE: typeof ThreeNS,
    OrbitControls: typeof import('three/examples/jsm/controls/OrbitControls.js').OrbitControls
  ): Engine {
    const el = host!;
    const cv = canvas!;
    const renderer = new THREE.WebGLRenderer({
      canvas: cv,
      antialias: true,
      alpha: true,
      powerPreference: 'low-power',
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.setClearColor(0x000000, 0);

    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x100904, 0.028);
    const camera = new THREE.PerspectiveCamera(48, 1, 0.1, 200);
    camera.position.set(0, 3, 17);

    const controls = new OrbitControls(camera, cv);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 4;
    controls.maxDistance = 34;
    controls.rotateSpeed = 0.7;
    controls.zoomSpeed = 0.8;
    controls.enablePan = false;
    controls.autoRotateSpeed = 0.45;

    scene.add(new THREE.AmbientLight(0xffe6c8, 0.65));
    const key = new THREE.PointLight(0xffd2a0, 2.4, 60, 1.4);
    scene.add(key);
    const rim = new THREE.DirectionalLight(0xff7a2e, 0.5);
    rim.position.set(-6, 4, -8);
    scene.add(rim);

    // Soft glow sprite texture, shared by every node.
    const glowCanvas = document.createElement('canvas');
    glowCanvas.width = glowCanvas.height = 128;
    const gctx = glowCanvas.getContext('2d')!;
    const grad = gctx.createRadialGradient(64, 64, 0, 64, 64, 64);
    grad.addColorStop(0, 'rgba(255,255,255,0.95)');
    grad.addColorStop(0.35, 'rgba(255,255,255,0.32)');
    grad.addColorStop(1, 'rgba(255,255,255,0)');
    gctx.fillStyle = grad;
    gctx.fillRect(0, 0, 128, 128);
    const glowTex = new THREE.CanvasTexture(glowCanvas);

    // ── Dust ────────────────────────────────────────────────
    const dustGeo = new THREE.BufferGeometry();
    {
      const n = 360;
      const arr = new Float32Array(n * 3);
      let s = 1337;
      const rnd = () => {
        s = (s * 16807) % 2147483647;
        return s / 2147483647;
      };
      for (let i = 0; i < n; i++) {
        const r = 6 + rnd() * 16;
        const th = rnd() * Math.PI * 2;
        const ph = Math.acos(2 * rnd() - 1);
        arr[i * 3] = r * Math.sin(ph) * Math.cos(th);
        arr[i * 3 + 1] = r * Math.cos(ph) * 0.7;
        arr[i * 3 + 2] = r * Math.sin(ph) * Math.sin(th);
      }
      dustGeo.setAttribute('position', new THREE.BufferAttribute(arr, 3));
    }
    const dust = new THREE.Points(
      dustGeo,
      new THREE.PointsMaterial({
        color: 0xffe0bd,
        size: 0.05,
        transparent: true,
        opacity: 0.45,
        depthWrite: false,
      })
    );
    scene.add(dust);

    // ── Graph (rebuilt when the data changes) ───────────────
    type NodeObj = {
      id: string;
      mesh: ThreeNS.Mesh;
      glow: ThreeNS.Sprite;
      base: number;
      color: ThreeNS.Color;
      pos: ThreeNS.Vector3;
      phase: number;
      // eased visual state
      emphasis: number;
      dim: number;
    };
    type EdgeObj = {
      from: string;
      to: string;
      line: ThreeNS.Line;
      curve: ThreeNS.CatmullRomCurve3;
      pulse: ThreeNS.Sprite;
      t: number;
      level: number;
    };
    const graph = new THREE.Group();
    scene.add(graph);
    let nodeObjs = new Map<string, NodeObj>();
    let edgeObjs: EdgeObj[] = [];
    const disposables: { dispose(): void }[] = [];
    const sphereGeo = new THREE.SphereGeometry(1, 36, 26);
    disposables.push(sphereGeo);

    function clearGraph() {
      graph.clear();
      for (const d of disposables.splice(0)) d.dispose();
      disposables.push(sphereGeo);
      nodeObjs = new Map();
      edgeObjs = [];
    }

    function build() {
      clearGraph();
      const byId = new Map(nodes.map((n) => [n.id, n]));
      for (const n of nodes) {
        const color = new THREE.Color(colorForGroup(groups, n.group));
        const mat = new THREE.MeshStandardMaterial({
          color,
          emissive: color,
          emissiveIntensity: 0.35,
          roughness: 0.42,
          metalness: 0.12,
        });
        disposables.push(mat);
        const mesh = new THREE.Mesh(sphereGeo, mat);
        mesh.scale.setScalar(n.r);
        mesh.position.set(n.x, n.y, n.z);
        mesh.userData.id = n.id;
        graph.add(mesh);

        const gmat = new THREE.SpriteMaterial({
          map: glowTex,
          color,
          transparent: true,
          opacity: 0.35,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
        });
        disposables.push(gmat);
        const glow = new THREE.Sprite(gmat);
        glow.position.copy(mesh.position);
        glow.scale.setScalar(n.r * 5);
        graph.add(glow);

        nodeObjs.set(n.id, {
          id: n.id,
          mesh,
          glow,
          base: n.r,
          color,
          pos: mesh.position.clone(),
          phase: (n.x * 1.7 + n.z) % (Math.PI * 2),
          emphasis: 0,
          dim: 0,
        });
      }

      for (const e of edges) {
        const a = byId.get(e.from)!;
        const b = byId.get(e.to)!;
        const pa = new THREE.Vector3(a.x, a.y, a.z);
        const pb = new THREE.Vector3(b.x, b.y, b.z);
        const mid = pa.clone().add(pb).multiplyScalar(0.5);
        // Bow the edge away from the centre so lines read as arcs.
        mid.add(
          mid
            .clone()
            .normalize()
            .multiplyScalar(pa.distanceTo(pb) * 0.16)
        );
        const curve = new THREE.CatmullRomCurve3([pa, mid, pb]);
        const geo = new THREE.BufferGeometry().setFromPoints(
          curve.getPoints(36)
        );
        const mat = new THREE.LineBasicMaterial({
          color: 0xffedd7,
          transparent: true,
          opacity: 0.2,
        });
        disposables.push(geo, mat);
        const line = new THREE.Line(geo, mat);
        graph.add(line);

        const pmat = new THREE.SpriteMaterial({
          map: glowTex,
          color: 0xffb070,
          transparent: true,
          opacity: 0,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
        });
        disposables.push(pmat);
        const pulse = new THREE.Sprite(pmat);
        pulse.scale.setScalar(0.55);
        graph.add(pulse);
        edgeObjs.push({
          from: e.from,
          to: e.to,
          line,
          curve,
          pulse,
          t: Math.random(),
          level: 0,
        });
      }
      refreshTargets();
    }

    // ── State ───────────────────────────────────────────────
    let selected: string | null = null;
    let hovered: string | null = null;
    let active = new Set<string>();
    let following = false;
    let reduced = false;
    let disposed = false;
    let lastInteraction = performance.now();

    const neighbours = (id: string) => {
      const out = new Set<string>([id]);
      for (const e of edges) {
        if (e.from === id) out.add(e.to);
        if (e.to === id) out.add(e.from);
      }
      return out;
    };

    let focusSet: Set<string> | null = null;
    function refreshTargets() {
      const focusId = hovered ?? selected;
      focusSet = focusId ? neighbours(focusId) : null;
    }

    // ── Camera tween ────────────────────────────────────────
    let tween: {
      t0: number;
      dur: number;
      fromPos: ThreeNS.Vector3;
      toPos: ThreeNS.Vector3;
      fromTarget: ThreeNS.Vector3;
      toTarget: ThreeNS.Vector3;
    } | null = null;

    const ease = (t: number) => 1 - Math.pow(1 - t, 3);

    function flyTo(target: ThreeNS.Vector3, distance: number, dur = 850) {
      const dir = camera.position.clone().sub(controls.target).normalize();
      const toPos = target.clone().add(dir.multiplyScalar(distance));
      if (reduced) {
        camera.position.copy(toPos);
        controls.target.copy(target);
        controls.update();
        return;
      }
      tween = {
        t0: performance.now(),
        dur,
        fromPos: camera.position.clone(),
        toPos,
        fromTarget: controls.target.clone(),
        toTarget: target.clone(),
      };
    }

    const fitDistance = () => {
      let max = 1;
      for (const n of nodes)
        max = Math.max(max, Math.hypot(n.x, n.y, n.z) + n.r);
      const vFov = (camera.fov * Math.PI) / 180;
      const hFov = 2 * Math.atan(Math.tan(vFov / 2) * camera.aspect);
      return (max * 0.96) / Math.sin(Math.min(vFov, hFov) / 2);
    };

    /** A fixed, comfortable close-up distance, so repeated fly-tos (a tour)
     *  never compound into ever tighter zooms. */
    const flyDistance = () =>
      Math.min(Math.max(fitDistance() * 0.68, 8), controls.maxDistance);

    // ── Sizing ──────────────────────────────────────────────
    function resize() {
      const w = Math.max(el.clientWidth, 1);
      const h = Math.max(el.clientHeight, 1);
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    const ro = new ResizeObserver(() => {
      resize();
      if (!hasFitted) fitNow();
    });
    ro.observe(el);

    let hasFitted = false;
    function fitNow() {
      resize();
      const d = fitDistance();
      const dir = new THREE.Vector3(0.18, 0.28, 1).normalize();
      camera.position.copy(dir.multiplyScalar(d));
      controls.target.set(0, 0, 0);
      controls.update();
      hasFitted = true;
    }

    // ── Picking ─────────────────────────────────────────────
    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    let downAt: { x: number; y: number } | null = null;

    function pick(clientX: number, clientY: number): string | null {
      const rect = cv.getBoundingClientRect();
      pointer.set(
        ((clientX - rect.left) / rect.width) * 2 - 1,
        -((clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(pointer, camera);
      const hits = raycaster.intersectObjects(
        [...nodeObjs.values()].map((n) => n.mesh),
        false
      );
      return hits[0]?.object.userData.id ?? null;
    }

    function onPointerMove(e: PointerEvent) {
      if (e.buttons) return;
      const id = pick(e.clientX, e.clientY);
      if (id !== hovered) {
        hovered = id;
        cv.style.cursor = id ? 'pointer' : 'grab';
        refreshTargets();
      }
    }
    function onPointerLeave() {
      hovered = null;
      refreshTargets();
    }
    function onPointerDown(e: PointerEvent) {
      downAt = { x: e.clientX, y: e.clientY };
      lastInteraction = performance.now();
      tween = null;
    }
    function onPointerUp(e: PointerEvent) {
      lastInteraction = performance.now();
      if (!downAt) return;
      const moved = Math.hypot(e.clientX - downAt.x, e.clientY - downAt.y);
      downAt = null;
      if (moved > 5) return;
      const id = pick(e.clientX, e.clientY);
      onSelect?.(id);
    }
    function onWheel() {
      lastInteraction = performance.now();
      tween = null;
    }
    cv.addEventListener('pointermove', onPointerMove);
    cv.addEventListener('pointerleave', onPointerLeave);
    cv.addEventListener('pointerdown', onPointerDown);
    cv.addEventListener('pointerup', onPointerUp);
    cv.addEventListener('wheel', onWheel, { passive: true });
    cv.style.cursor = 'grab';

    // ── Visibility: stop drawing when off-screen or hidden ──
    let onScreen = true;
    const io = new IntersectionObserver(
      (entries) => (onScreen = entries[0]?.isIntersecting ?? true)
    );
    io.observe(el);

    // ── Frame loop ──────────────────────────────────────────
    const tmp = new THREE.Vector3();
    const tmp2 = new THREE.Vector3();
    let last = performance.now();
    let raf = 0;

    function frame(now: number) {
      raf = requestAnimationFrame(frame);
      if (disposed || !onScreen || document.hidden) {
        last = now;
        return;
      }
      const dt = Math.min((now - last) / 1000, 0.1);
      last = now;
      const time = now / 1000;

      // Camera tween
      if (tween) {
        const k = Math.min((now - tween.t0) / tween.dur, 1);
        const e = ease(k);
        camera.position.lerpVectors(tween.fromPos, tween.toPos, e);
        controls.target.lerpVectors(tween.fromTarget, tween.toTarget, e);
        if (k >= 1) tween = null;
      } else if (following && active.size && !reduced) {
        // Drift toward the centroid of what is being discussed.
        tmp.set(0, 0, 0);
        let c = 0;
        for (const id of active) {
          const o = nodeObjs.get(id);
          if (o) {
            tmp.add(o.pos);
            c++;
          }
        }
        if (c) {
          tmp.multiplyScalar(1 / c);
          tmp2
            .copy(tmp)
            .sub(controls.target)
            .multiplyScalar(Math.min(dt * 1.6, 1));
          controls.target.add(tmp2);
          camera.position.add(tmp2);
        }
      }

      // Idle rotation (never with reduced motion, or while something is picked)
      controls.autoRotate =
        !reduced && !selected && now - lastInteraction > 5000 && !following;
      controls.update();
      key.position.copy(camera.position);

      // Nodes
      for (const o of nodeObjs.values()) {
        const inFocus = !focusSet || focusSet.has(o.id);
        const isSel = o.id === selected;
        const isHover = o.id === hovered;
        const isActive = active.has(o.id);
        const targetEmph = isSel ? 1 : isHover ? 0.75 : isActive ? 0.55 : 0;
        const targetDim = inFocus ? 0 : 1;
        o.emphasis += (targetEmph - o.emphasis) * Math.min(dt * 9, 1);
        o.dim += (targetDim - o.dim) * Math.min(dt * 9, 1);
        const pulse =
          isActive && !reduced ? 1 + Math.sin(time * 4 + o.phase) * 0.1 : 1;
        const bob = reduced ? 0 : Math.sin(time * 0.8 + o.phase) * 0.05;
        const s = o.base * (1 + o.emphasis * 0.28) * pulse;
        o.mesh.scale.setScalar(s);
        o.mesh.position.set(o.pos.x, o.pos.y + bob, o.pos.z);
        o.glow.position.copy(o.mesh.position);
        o.glow.scale.setScalar(s * (4.6 + o.emphasis * 2.2));
        const mat = o.mesh.material as ThreeNS.MeshStandardMaterial;
        mat.emissiveIntensity = (0.3 + o.emphasis * 0.75) * (1 - o.dim * 0.7);
        mat.opacity = 1;
        (o.glow.material as ThreeNS.SpriteMaterial).opacity =
          (0.28 + o.emphasis * 0.5) * (1 - o.dim * 0.85);
        mat.color.copy(o.color).multiplyScalar(1 - o.dim * 0.55);
      }

      // Edges and pulses
      for (const ed of edgeObjs) {
        const related =
          (selected || hovered) &&
          ((hovered ?? selected) === ed.from ||
            (hovered ?? selected) === ed.to);
        const activeEdge = active.has(ed.from) && active.has(ed.to);
        const target = related
          ? 0.85
          : activeEdge
            ? 0.55
            : focusSet
              ? 0.05
              : 0.2;
        ed.level += (target - ed.level) * Math.min(dt * 8, 1);
        (ed.line.material as ThreeNS.LineBasicMaterial).opacity = ed.level;
        const showPulse = (related || activeEdge) && !reduced;
        const pm = ed.pulse.material as ThreeNS.SpriteMaterial;
        if (showPulse) {
          ed.t = (ed.t + dt * 0.45) % 1;
          ed.curve.getPoint(ed.t, tmp);
          ed.pulse.position.copy(tmp);
          pm.opacity += (0.9 - pm.opacity) * Math.min(dt * 8, 1);
        } else {
          pm.opacity += (0 - pm.opacity) * Math.min(dt * 8, 1);
        }
      }

      if (!reduced) dust.rotation.y += dt * 0.012;

      renderer.render(scene, camera);
      positionLabels();
    }

    // ── DOM labels (real text, so size/contrast follow the page) ──
    const wv = new THREE.Vector3();
    const sizeCache = new Map<string, { w: number; h: number }>();
    type Placed = { x: number; y: number; w: number; h: number };
    function positionLabels() {
      const w = el.clientWidth;
      const h = el.clientHeight;
      const items: {
        o: NodeObj;
        lab: HTMLElement;
        x: number;
        y: number;
        behind: boolean;
        depth: number;
        priority: number;
        forced: boolean;
      }[] = [];
      for (const o of nodeObjs.values()) {
        const lab = labelEls[o.id];
        if (!lab) continue;
        wv.copy(o.mesh.position).project(camera);
        const x = (wv.x * 0.5 + 0.5) * w;
        const dist = Math.max(camera.position.distanceTo(o.mesh.position), 0.1);
        // Sit the label just under the sphere's on-screen edge.
        const radiusPx =
          ((o.mesh.scale.x / dist) * (h / 2)) /
          Math.tan((camera.fov * Math.PI) / 360);
        const y = (-wv.y * 0.5 + 0.5) * h + radiusPx + 8;
        const depth = Math.min(Math.max((wv.z - 0.85) / 0.15, 0), 1);
        const forced = o.id === selected || o.id === hovered;
        const imp = (nodes.find((n) => n.id === o.id)?.importance ??
          1) as number;
        items.push({
          o,
          lab,
          x,
          y,
          behind: wv.z > 1,
          depth,
          forced,
          priority:
            (forced ? 1000 : 0) +
            (active.has(o.id) ? 500 : 0) +
            imp * 10 -
            depth * 5,
        });
      }
      // Greedy declutter: highest priority first; later labels that would
      // overlap an already-placed one fade out instead of stacking.
      items.sort((a, b) => b.priority - a.priority);
      const placed: Placed[] = [];
      for (const it of items) {
        let size = sizeCache.get(it.o.id);
        if (!size || size.w === 0) {
          size = { w: it.lab.offsetWidth, h: it.lab.offsetHeight };
          sizeCache.set(it.o.id, size);
        }
        const rect = {
          x: it.x - size.w / 2 - 4,
          y: it.y - 2,
          w: size.w + 8,
          h: size.h + 4,
        };
        const hit = placed.some(
          (p) =>
            rect.x < p.x + p.w &&
            rect.x + rect.w > p.x &&
            rect.y < p.y + p.h &&
            rect.y + rect.h > p.y
        );
        const show = !it.behind && (it.forced || !hit);
        if (show) placed.push(rect);
        const base = 1 - it.depth * 0.5;
        const opacity = show
          ? it.forced
            ? 1
            : base * (1 - it.o.dim * 0.75)
          : 0;
        it.lab.style.transform = `translate3d(${it.x.toFixed(1)}px, ${it.y.toFixed(1)}px, 0) translate(-50%, 0)`;
        it.lab.style.opacity = opacity.toFixed(2);
        it.lab.style.zIndex = String(Math.round(it.priority + 2000));
      }
    }

    // ── Public API ──────────────────────────────────────────
    let builtKey = `${visualization.id}:${nodes.length}`;
    build();
    fitNow();
    raf = requestAnimationFrame(frame);

    const api: Engine = {
      setSelected(id) {
        selected = id;
        refreshTargets();
        if (id) {
          const o = nodeObjs.get(id);
          if (o) flyTo(o.pos, flyDistance());
        }
        lastInteraction = performance.now();
      },
      setActive(ids) {
        active = ids;
      },
      setFollow(on) {
        following = on;
      },
      setReduced(on) {
        reduced = on;
        if (on) controls.autoRotate = false;
      },
      reset() {
        if (!hasFitted) return;
        const d = fitDistance();
        const dir = new THREE.Vector3(0.18, 0.28, 1).normalize();
        const center = new THREE.Vector3(0, 0, 0);
        const toPos = dir.multiplyScalar(d);
        if (reduced) {
          camera.position.copy(toPos);
          controls.target.copy(center);
          controls.update();
        } else {
          tween = {
            t0: performance.now(),
            dur: 900,
            fromPos: camera.position.clone(),
            toPos,
            fromTarget: controls.target.clone(),
            toTarget: center,
          };
        }
      },
      zoom(dir) {
        const offset = camera.position.clone().sub(controls.target);
        const next = Math.min(
          Math.max(
            offset.length() * (dir > 0 ? 0.8 : 1.25),
            controls.minDistance
          ),
          controls.maxDistance
        );
        offset.setLength(next);
        const toPos = controls.target.clone().add(offset);
        if (reduced) {
          camera.position.copy(toPos);
          controls.update();
        } else {
          tween = {
            t0: performance.now(),
            dur: 260,
            fromPos: camera.position.clone(),
            toPos,
            fromTarget: controls.target.clone(),
            toTarget: controls.target.clone(),
          };
        }
      },
      rebuild(key) {
        if (key === builtKey) return;
        builtKey = key;
        build();
        if (selected && !nodeObjs.has(selected)) selected = null;
        refreshTargets();
      },
      dispose() {
        disposed = true;
        cancelAnimationFrame(raf);
        ro.disconnect();
        io.disconnect();
        cv.removeEventListener('pointermove', onPointerMove);
        cv.removeEventListener('pointerleave', onPointerLeave);
        cv.removeEventListener('pointerdown', onPointerDown);
        cv.removeEventListener('pointerup', onPointerUp);
        cv.removeEventListener('wheel', onWheel);
        controls.dispose();
        clearGraph();
        sphereGeo.dispose();
        dustGeo.dispose();
        (dust.material as ThreeNS.Material).dispose();
        glowTex.dispose();
        renderer.dispose();
        renderer.forceContextLoss();
      },
    };
    return api;
  }
</script>

<div class="stage3d" bind:this={host}>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>

  <!-- Labels are real DOM text so they scale and meet contrast with the page.
       They are decorative here: the concept list is the accessible equivalent. -->
  <div class="labels" aria-hidden="true">
    {#each nodes as node (node.id)}
      <span
        class="node-label"
        class:active={activeIds.has(node.id)}
        class:selected={selectedId === node.id}
        bind:this={labelEls[node.id]}
        style:--size="{11.5 + Math.min(node.importance, 6) * 0.9}px"
      >
        {node.label}
      </span>
    {/each}
  </div>

  {#if !ready}
    <div class="boot" role="status">Preparing the 3D view…</div>
  {/if}
</div>

<style>
  .stage3d {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 240px;
    overflow: hidden;
    border-radius: var(--radius-lg);
    background:
      radial-gradient(
        120% 90% at 50% 40%,
        rgba(74, 40, 18, 0.55),
        rgba(16, 9, 4, 0.95) 70%
      ),
      #100904;
    border: 1px solid var(--border-subtle);
    touch-action: none;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    outline: none;
  }
  .labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .node-label {
    position: absolute;
    left: 0;
    top: 0;
    white-space: nowrap;
    font-family: var(--font-body);
    font-size: var(--size);
    font-weight: 600;
    letter-spacing: 0.01em;
    color: var(--text-primary);
    text-shadow:
      0 1px 3px rgba(0, 0, 0, 0.9),
      0 0 12px rgba(16, 9, 4, 0.95);
    will-change: transform, opacity;
  }
  .node-label.active {
    color: #ffd2a8;
  }
  .node-label.selected {
    color: #fff;
  }
  .boot {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
