<script lang="ts">
	import { onMount } from 'svelte';

	let canvas: HTMLCanvasElement;
	let blurDiv: HTMLDivElement;

	onMount(() => {
		const ctx = canvas.getContext('2d')!;
		const w = (canvas.width = window.innerWidth);
		const h = (canvas.height = window.innerHeight);

		// Static crack layer — rendered once, composited ON TOP of green bars
		const crackLayer = document.createElement('canvas');
		crackLayer.width = w;
		crackLayer.height = h;
		const cctx = crackLayer.getContext('2d')!;

		// Mask canvas — white on black along crack paths,
		// drives the CSS backdrop-filter div
		const maskCanvas = document.createElement('canvas');
		maskCanvas.width = w;
		maskCanvas.height = h;
		const mctx = maskCanvas.getContext('2d')!;
		mctx.fillStyle = 'black';
		mctx.fillRect(0, 0, w, h);

		const impact = { x: w * 0.63, y: h * 0.36 };

		type Pt = { x: number; y: number };

		// ── CRACK GENERATION ──────────────────────────────
		function makeCrack(
			ox: number,
			oy: number,
			angle: number,
			maxLen: number,
			jitter: number,
			step = 9
		): Pt[] {
			const pts: Pt[] = [{ x: ox, y: oy }];
			let x = ox,
				y = oy,
				a = angle;
			const n = Math.floor(maxLen / step);
			for (let i = 0; i < n; i++) {
				a += (Math.random() - 0.5) * jitter;
				if (Math.random() < 0.08) a += (Math.random() < 0.5 ? 1 : -1) * (0.5 + Math.random() * 0.6);
				x += Math.cos(a) * step;
				y += Math.sin(a) * step;
				pts.push({ x, y });
				if (x < -30 || x > w + 30 || y < -30 || y > h + 30) break;
			}
			return pts;
		}

		const mainAngles: number[] = [];
		let sa = Math.random() * Math.PI * 2;
		for (let i = 0; i < 9; i++) {
			sa += ((Math.PI * 2) / 9) * (0.5 + Math.random() * 1.0);
			mainAngles.push(sa);
		}
		const mainCracks: Pt[][] = mainAngles.map((a) =>
			makeCrack(impact.x, impact.y, a, Math.hypot(w, h) * 0.8, 0.18)
		);

		const branches: Pt[][] = [];
		for (const mc of mainCracks) {
			for (let i = 5; i < mc.length; i += 9 + Math.floor(Math.random() * 12)) {
				if (Math.random() < 0.52) {
					const pa = Math.atan2(mc[i].y - mc[i - 1].y, mc[i].x - mc[i - 1].x);
					const ba = pa + (Math.random() < 0.5 ? 1 : -1) * (0.3 + Math.random() * 0.65);
					branches.push(makeCrack(mc[i].x, mc[i].y, ba, 50 + Math.random() * 160, 0.28, 7));
				}
			}
		}

		const rings: Pt[][] = [];
		for (const ringR of [60, 130, 230, 380, 560]) {
			for (let ci = 0; ci < mainCracks.length; ci++) {
				const cA = mainCracks[ci];
				const cB = mainCracks[(ci + 1) % mainCracks.length];
				let ptA: Pt | null = null,
					ptB: Pt | null = null;
				let bestA = Infinity,
					bestB = Infinity;
				for (const p of cA) {
					const d = Math.abs(Math.hypot(p.x - impact.x, p.y - impact.y) - ringR);
					if (d < bestA) {
						bestA = d;
						ptA = p;
					}
				}
				for (const p of cB) {
					const d = Math.abs(Math.hypot(p.x - impact.x, p.y - impact.y) - ringR);
					if (d < bestB) {
						bestB = d;
						ptB = p;
					}
				}
				if (!ptA || !ptB) continue;
				const angA = Math.atan2(ptA.y - impact.y, ptA.x - impact.x);
				const angB = Math.atan2(ptB.y - impact.y, ptB.x - impact.x);
				let dAng = angB - angA;
				if (dAng > Math.PI) dAng -= Math.PI * 2;
				if (dAng < -Math.PI) dAng += Math.PI * 2;
				if (Math.abs(dAng) > Math.PI * 0.78) continue;
				const steps = Math.max(5, Math.floor((Math.abs(dAng) * ringR) / 22));
				const arcPts: Pt[] = [{ x: ptA.x, y: ptA.y }];
				for (let s = 1; s < steps; s++) {
					const t = s / steps,
						ang = angA + dAng * t;
					const r = ringR * (0.88 + Math.random() * 0.22);
					arcPts.push({
						x: impact.x + Math.cos(ang) * r + (Math.random() - 0.5) * 10,
						y: impact.y + Math.sin(ang) * r + (Math.random() - 0.5) * 10
					});
				}
				arcPts.push({ x: ptB.x, y: ptB.y });
				rings.push(arcPts);
			}
		}

		// ── MASK CANVAS — crack paths for backdrop-filter ──
		// Feathered white strokes along every crack path.
		// The soft outer pass creates a blur halo around each crack edge.
		function strokeMask(pts: Pt[], width: number) {
			if (pts.length < 2) return;
			// Soft outer feather
			mctx.beginPath();
			mctx.moveTo(pts[0].x, pts[0].y);
			for (let i = 1; i < pts.length; i++) mctx.lineTo(pts[i].x, pts[i].y);
			mctx.strokeStyle = 'rgba(255,255,255,0.35)';
			mctx.lineWidth = width * 3.5;
			mctx.lineCap = 'round';
			mctx.lineJoin = 'round';
			mctx.stroke();
			// Solid core
			mctx.beginPath();
			mctx.moveTo(pts[0].x, pts[0].y);
			for (let i = 1; i < pts.length; i++) mctx.lineTo(pts[i].x, pts[i].y);
			mctx.strokeStyle = 'rgba(255,255,255,0.90)';
			mctx.lineWidth = width;
			mctx.stroke();
		}

		// Blurred disc around impact point in mask
		const mig = mctx.createRadialGradient(impact.x, impact.y, 0, impact.x, impact.y, 90);
		mig.addColorStop(0, 'rgba(255,255,255,0.85)');
		mig.addColorStop(0.5, 'rgba(255,255,255,0.40)');
		mig.addColorStop(1, 'rgba(255,255,255,0)');
		mctx.fillStyle = mig;
		mctx.fillRect(impact.x - 90, impact.y - 90, 180, 180);

		for (const r of rings) strokeMask(r, 3);
		for (const b of branches) strokeMask(b, 5);
		for (const mc of mainCracks) strokeMask([{ x: impact.x, y: impact.y }, ...mc], 10);

		// Apply mask to the Svelte-bound blur div
		const maskURL = maskCanvas.toDataURL();
		blurDiv.style.webkitMaskImage = `url(${maskURL})`;
		blurDiv.style.maskImage = `url(${maskURL})`;
		blurDiv.style.webkitMaskSize = '100% 100%';
		blurDiv.style.maskSize = '100% 100%';

		// ── STATIC CRACK LAYER ────────────────────────────

		// Subtler dark zone — smaller radius, lighter opacities
		{
			const g = cctx.createRadialGradient(impact.x, impact.y, 0, impact.x, impact.y, 110);
			g.addColorStop(0, 'rgba(0,0,0,0.82)');
			g.addColorStop(0.35, 'rgba(0,0,0,0.55)');
			g.addColorStop(0.65, 'rgba(0,0,0,0.22)');
			g.addColorStop(0.88, 'rgba(0,0,0,0.06)');
			g.addColorStop(1, 'rgba(0,0,0,0)');
			cctx.fillStyle = g;
			cctx.fillRect(0, 0, w, h);
		}

		// Faint cold shard tinting
		const shardTints: [number, number, number, number][] = [
			[200, 210, 230, 0.025],
			[215, 220, 240, 0.035],
			[185, 200, 225, 0.02],
			[210, 218, 235, 0.03],
			[175, 195, 220, 0.016],
			[205, 215, 232, 0.027]
		];
		for (let i = 0; i < mainCracks.length; i++) {
			const cA = mainCracks[i];
			const cB = mainCracks[(i + 1) % mainCracks.length];
			const nA = Math.min(cA.length, 20 + Math.floor(Math.random() * 12));
			const nB = Math.min(cB.length, 20 + Math.floor(Math.random() * 12));
			cctx.beginPath();
			cctx.moveTo(impact.x, impact.y);
			for (let j = 0; j < nA; j++) cctx.lineTo(cA[j].x, cA[j].y);
			for (let j = nB - 1; j >= 0; j--) cctx.lineTo(cB[j].x, cB[j].y);
			cctx.closePath();
			const [r, g, b, a] = shardTints[i % shardTints.length];
			cctx.fillStyle = `rgba(${r},${g},${b},${a})`;
			cctx.fill();
		}

		// Solid polyline cracks — no per-segment stroke (prevents dotting)
		cctx.shadowBlur = 0;

		function drawCrackSolid(
			pts: Pt[],
			thickW: number,
			thinW: number,
			darkAlpha: number,
			brightAlpha: number
		) {
			if (pts.length < 2) return;
			const mid = Math.floor(pts.length * 0.35);

			// Dark void — thick near origin
			cctx.beginPath();
			cctx.moveTo(pts[0].x, pts[0].y);
			for (let i = 1; i <= Math.min(mid, pts.length - 1); i++) cctx.lineTo(pts[i].x, pts[i].y);
			cctx.strokeStyle = `rgba(0,0,0,${darkAlpha})`;
			cctx.lineWidth = thickW + 1.2;
			cctx.lineCap = 'butt';
			cctx.lineJoin = 'round';
			cctx.stroke();

			// Dark void — thin toward tip
			if (mid < pts.length - 1) {
				cctx.beginPath();
				cctx.moveTo(pts[mid].x, pts[mid].y);
				for (let i = mid + 1; i < pts.length; i++) cctx.lineTo(pts[i].x, pts[i].y);
				cctx.strokeStyle = `rgba(0,0,0,${darkAlpha * 0.72})`;
				cctx.lineWidth = thinW + 0.6;
				cctx.lineCap = 'round';
				cctx.stroke();
			}

			// Bright white refraction edge — boosted vs previous versions
			cctx.beginPath();
			cctx.moveTo(pts[0].x, pts[0].y);
			for (let i = 1; i < pts.length; i++) cctx.lineTo(pts[i].x, pts[i].y);
			cctx.strokeStyle = `rgba(255,255,255,${brightAlpha})`;
			cctx.lineWidth = Math.max(0.5, thinW * 0.45);
			cctx.lineCap = 'round';
			cctx.lineJoin = 'round';
			cctx.stroke();
		}

		//                                   thickW thinW darkA  brightA
		for (const r of rings) drawCrackSolid(r, 1.0, 0.4, 0.85, 0.5);
		for (const b of branches) drawCrackSolid(b, 2.0, 0.5, 0.88, 0.62);
		for (const mc of mainCracks)
			drawCrackSolid([{ x: impact.x, y: impact.y }, ...mc], 5.5, 0.6, 0.9, 0.8);

		// Impact centre
		{
			const g = cctx.createRadialGradient(impact.x, impact.y, 0, impact.x, impact.y, 55);
			g.addColorStop(0, 'rgba(255,255,255,0.96)');
			g.addColorStop(0.22, 'rgba(245,250,255,0.65)');
			g.addColorStop(0.55, 'rgba(230,238,255,0.18)');
			g.addColorStop(1, 'rgba(255,255,255,0)');
			cctx.fillStyle = g;
			cctx.fillRect(impact.x - 55, impact.y - 55, 110, 110);
		}
		cctx.beginPath();
		cctx.arc(impact.x, impact.y, 5, 0, Math.PI * 2);
		cctx.fillStyle = 'white';
		cctx.fill();

		for (let i = 0; i < 20; i++) {
			const ma = Math.random() * Math.PI * 2,
				ml = 7 + Math.random() * 26;
			cctx.beginPath();
			cctx.moveTo(impact.x, impact.y);
			cctx.lineTo(impact.x + Math.cos(ma) * ml, impact.y + Math.sin(ma) * ml);
			cctx.strokeStyle = `rgba(255,255,255,${0.45 + Math.random() * 0.5})`;
			cctx.lineWidth = 0.4 + Math.random() * 0.7;
			cctx.stroke();
		}

		// ── GREEN BARS ────────────────────────────────────
		type FlickerStyle = 'fast' | 'slow' | 'pulse';
		interface GreenBar {
			x: number;
			w: number;
			startY: number;
			endY: number;
			style: FlickerStyle;
			baseAlpha: number;
			timer: number;
			onDur: number;
			offDur: number;
			visible: boolean;
			pulsePhase: number;
			r: number;
			g: number;
			b: number;
		}

		function makeBar(x: number, style: FlickerStyle): GreenBar {
			const startY = Math.random() < 0.45 ? Math.floor(Math.random() * h * 0.45) : 0;
			const endY = Math.random() < 0.35 ? Math.floor(startY + h * (0.15 + Math.random() * 0.6)) : h;
			return {
				x: Math.floor(x),
				w: Math.random() < 0.78 ? 1 : 1 + Math.floor(Math.random() * 3),
				startY,
				endY,
				style,
				baseAlpha: 0.4 + Math.random() * 0.4,
				timer: Math.random() * 220,
				onDur:
					style === 'fast'
						? 2 + Math.random() * 8
						: style === 'slow'
							? 30 + Math.random() * 90
							: 15 + Math.random() * 40,
				offDur:
					style === 'fast'
						? 1 + Math.random() * 12
						: style === 'slow'
							? 20 + Math.random() * 120
							: 8 + Math.random() * 25,
				visible: Math.random() < 0.5,
				pulsePhase: Math.random() * Math.PI * 2,
				r: Math.floor(10 + Math.random() * 30),
				g: Math.floor(205 + Math.random() * 50),
				b: Math.floor(Math.random() * 28)
			};
		}

		const greenBars: GreenBar[] = [];
		const styles: FlickerStyle[] = ['fast', 'fast', 'slow', 'pulse'];

		for (const mc of mainCracks) {
			for (let i = 4; i < mc.length; i += 8 + Math.floor(Math.random() * 12)) {
				if (Math.random() < 0.22)
					greenBars.push(makeBar(mc[i].x, styles[Math.floor(Math.random() * styles.length)]));
			}
		}
		for (let i = 0; i < 3; i++) {
			const ang = Math.random() * Math.PI * 2,
				d = 60 + Math.random() * 380;
			const bar = makeBar(impact.x + Math.cos(ang) * d, 'slow');
			bar.w = 2 + Math.floor(Math.random() * 4);
			bar.baseAlpha *= 0.7;
			greenBars.push(bar);
		}

		// ── ANIMATION LOOP ────────────────────────────────
		// Order: ① green bars ② crack layer on top
		let rafId: number;

		function animate() {
			ctx.clearRect(0, 0, w, h);

			for (const bar of greenBars) {
				bar.timer++;
				const dur = bar.visible ? bar.onDur : bar.offDur;
				if (bar.timer >= dur) {
					bar.visible = !bar.visible;
					bar.timer = 0;
					if (bar.visible) {
						bar.onDur =
							bar.style === 'fast'
								? 2 + Math.random() * 8
								: bar.style === 'slow'
									? 30 + Math.random() * 90
									: 15 + Math.random() * 40;
					} else {
						bar.offDur =
							bar.style === 'fast'
								? 1 + Math.random() * 12
								: bar.style === 'slow'
									? 20 + Math.random() * 120
									: 8 + Math.random() * 25;
					}
				}
				if (!bar.visible) continue;
				let alpha = bar.baseAlpha;
				if (bar.style === 'pulse') {
					bar.pulsePhase += 0.07 + Math.random() * 0.04;
					alpha *= 0.45 + 0.55 * Math.abs(Math.sin(bar.pulsePhase));
				} else if (bar.style === 'fast') {
					alpha *= 0.65 + Math.random() * 0.35;
				}
				ctx.globalAlpha = alpha;
				ctx.fillStyle = `rgb(${bar.r},${bar.g},${bar.b})`;
				ctx.fillRect(bar.x, bar.startY, bar.w, bar.endY - bar.startY);
			}
			ctx.globalAlpha = 1;

			ctx.drawImage(crackLayer, 0, 0);
			rafId = requestAnimationFrame(animate);
		}

		animate();
		return () => cancelAnimationFrame(rafId);
	});
</script>

<!--
	Stacking order:
	z-9997  #blur-div   — backdrop-filter blur, masked to crack paths
	z-9999  canvas      — green bars + crack layer drawn on top
  -->
<div
	bind:this={blurDiv}
	style="position:fixed;inset:0;z-index:9997;pointer-events:none;
		   backdrop-filter:blur(2.5px) brightness(1.08) contrast(1.05);
		   -webkit-backdrop-filter:blur(2.5px) brightness(1.08) contrast(1.05);"
></div>

<canvas bind:this={canvas} class="broken-monitor"></canvas>

<style>
	.broken-monitor {
		position: fixed;
		inset: 0;
		z-index: 9999;
		pointer-events: none;
	}
</style>
