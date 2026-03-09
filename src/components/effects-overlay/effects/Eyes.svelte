<script lang="ts">
	import { onMount } from 'svelte';

	let canvas: HTMLCanvasElement;

	// ── Utilities ─────────────────────────────────────
	function lerp(a: number, b: number, t: number) {
		return a + (b - a) * t;
	}
	// Frame-rate-independent lerp: halfLifeMs = time to close 50% of gap
	function lf(halfLifeMs: number, dt: number) {
		return 1 - Math.pow(0.5, dt / halfLifeMs);
	}

	onMount(() => {
		const w = (canvas.width = window.innerWidth);
		const h = (canvas.height = window.innerHeight);
		const ctx = canvas.getContext('2d')!;

		// ── Layout ──────────────────────────────────────
		const R = Math.min(105, w * 0.097); // eye socket radius
		const RY = R * 0.82; // vertical radius (slightly squashed)
		const SEP = R * 1.65; // half-gap between eye centres
		const ECY = h * 0.4; // eye centre Y
		const ECX = w / 2;

		const L = { cx: ECX - SEP, cy: ECY }; // left eye centre
		const Ri = { cx: ECX + SEP, cy: ECY }; // right eye centre

		// ── Animated state ───────────────────────────────
		const cur = {
			lookX: 0,
			lookY: 0, // -1…1 eye direction
			blinkL: 0,
			blinkR: 0, // 0=open 1=closed
			browLiftL: 0,
			browLiftR: 0, // px, neg=raised
			browTiltL: 0,
			browTiltR: 0, // radians
			irisScale: 1 // >1 = dilated (surprised)
		};
		const tgt = {
			lookX: 0,
			lookY: 0,
			browLiftL: 0,
			browLiftR: 0,
			browTiltL: 0,
			browTiltR: 0,
			irisScale: 1
		};

		// ── Expression presets ───────────────────────────
		type Expr = 'neutral' | 'surprised' | 'frown' | 'suspicious' | 'happy' | 'worried' | 'sleepy';
		const EXPR: Record<
			Expr,
			{
				browLiftL: number;
				browLiftR: number;
				browTiltL: number;
				browTiltR: number;
				irisScale: number;
			}
		> = {
			neutral: { browLiftL: 0, browLiftR: 0, browTiltL: 0.0, browTiltR: 0.0, irisScale: 1.0 },
			surprised: {
				browLiftL: -26,
				browLiftR: -26,
				browTiltL: 0.0,
				browTiltR: 0.0,
				irisScale: 1.12
			},
			frown: { browLiftL: -4, browLiftR: -4, browTiltL: 0.35, browTiltR: -0.35, irisScale: 0.95 },
			suspicious: {
				browLiftL: -6,
				browLiftR: 12,
				browTiltL: -0.2,
				browTiltR: 0.18,
				irisScale: 0.97
			},
			happy: { browLiftL: -16, browLiftR: -16, browTiltL: -0.12, browTiltR: 0.12, irisScale: 1.05 },
			worried: { browLiftL: 12, browLiftR: 12, browTiltL: 0.4, browTiltR: -0.4, irisScale: 1.02 },
			sleepy: { browLiftL: 8, browLiftR: 8, browTiltL: 0.05, browTiltR: -0.05, irisScale: 0.92 }
		};

		function applyExpr(e: Expr) {
			const p = EXPR[e];
			tgt.browLiftL = p.browLiftL;
			tgt.browLiftR = p.browLiftR;
			tgt.browTiltL = p.browTiltL;
			tgt.browTiltR = p.browTiltR;
			tgt.irisScale = p.irisScale;
		}

		// ── Blink state machine ──────────────────────────
		// Blink directly drives cur.blinkL/R (no lerp lag)
		let blinking = false;
		let blinkElapsed = 0;
		let blinkSleepy = false; // slow blink for sleepy expr
		const B_CLOSE = 70;
		const B_HOLD = 30;
		const B_OPEN = 90;

		function triggerBlink(slow = false) {
			if (blinking) return;
			blinking = true;
			blinkSleepy = slow;
			blinkElapsed = 0;
		}

		// ── Timing ──────────────────────────────────────
		let nextLookMs = 1200;
		let nextBlinkMs = 2800;
		let nextExprMs = 7000;
		let lastTs = 0,
			elapsed = 0;

		// Bias the expression pool so neutral is most common
		const EXPR_POOL: Expr[] = [
			'neutral',
			'neutral',
			'neutral',
			'surprised',
			'frown',
			'suspicious',
			'happy',
			'worried',
			'sleepy'
		];

		function newLookTarget() {
			const r = Math.random();
			if (r < 0.3) {
				tgt.lookX = (Math.random() - 0.5) * 1.9;
				tgt.lookY = (Math.random() - 0.5) * 1.9;
			} else if (r < 0.55) {
				tgt.lookX = (Math.random() - 0.5) * 0.22;
				tgt.lookY = (Math.random() - 0.5) * 0.22;
			} // stare at viewer
			else if (r < 0.72) {
				tgt.lookX = (Math.random() < 0.5 ? -1 : 1) * (0.75 + Math.random() * 0.25);
				tgt.lookY = (Math.random() - 0.5) * 0.45;
			} else {
				tgt.lookX = (Math.random() - 0.5) * 0.35;
				tgt.lookY = (Math.random() < 0.5 ? -1 : 1) * (0.65 + Math.random() * 0.3);
			}
			nextLookMs = 700 + Math.random() * 2400;
		}

		function newExpression() {
			const e = EXPR_POOL[Math.floor(Math.random() * EXPR_POOL.length)];
			applyExpr(e);
			// Pair look direction to expression
			if (e === 'surprised') {
				tgt.lookX = (Math.random() - 0.5) * 0.4;
				tgt.lookY = -0.65;
			}
			if (e === 'frown') {
				tgt.lookY = 0.4;
			}
			if (e === 'sleepy') {
				tgt.lookY = 0.3;
				triggerBlink(true);
			}
			nextExprMs = 5000 + Math.random() * 11000;
		}

		// ── Eye drawing ──────────────────────────────────
		function drawEye(
			cx: number,
			cy: number,
			lookX: number,
			lookY: number,
			blinkT: number,
			browLift: number,
			browTilt: number,
			irisScale: number,
			side: 'left' | 'right'
		) {
			const ry = RY;

			// 1. Dark vignette behind eye — makes it pop off any bg
			const vigG = ctx.createRadialGradient(cx, cy, R * 0.2, cx, cy, R * 2.2);
			vigG.addColorStop(0, 'rgba(0,0,10,0.55)');
			vigG.addColorStop(0.6, 'rgba(0,0,8,0.30)');
			vigG.addColorStop(1, 'rgba(0,0,0,0)');
			ctx.fillStyle = vigG;
			ctx.fillRect(cx - R * 2.3, cy - R * 2.3, R * 4.6, R * 4.6);

			// 2. Outer blue glow (eye is "alive")
			ctx.save();
			ctx.shadowColor = 'rgba(90, 155, 255, 0.45)';
			ctx.shadowBlur = 32;
			ctx.shadowOffsetY = 4;
			ctx.beginPath();
			ctx.ellipse(cx, cy, R, ry, 0, 0, Math.PI * 2);
			ctx.fillStyle = '#f2ede8';
			ctx.fill();
			ctx.restore();

			// 3. White sclera (clean, no shadow)
			ctx.beginPath();
			ctx.ellipse(cx, cy, R, ry, 0, 0, Math.PI * 2);
			ctx.fillStyle = '#f0ece7';
			ctx.fill();

			// 4. Clip everything to eye ellipse
			ctx.save();
			ctx.beginPath();
			ctx.ellipse(cx, cy, R, ry, 0, 0, Math.PI * 2);
			ctx.clip();

			// Pupil position — clamped inside iris boundary
			const irisR = 52 * irisScale;
			const maxOffX = (R - irisR * 0.7) * 0.55;
			const maxOffY = (ry - irisR * 0.7) * 0.55;
			const px = cx + lookX * maxOffX;
			const py = cy + lookY * maxOffY;

			// Iris gradient — blue (eerie & cute)
			const iG = ctx.createRadialGradient(px - 13, py - 13, 3, px, py, irisR);
			iG.addColorStop(0, '#c8ddff');
			iG.addColorStop(0.28, '#5e90f0');
			iG.addColorStop(0.65, '#2040a8');
			iG.addColorStop(1, '#080a50');
			ctx.beginPath();
			ctx.arc(px, py, irisR, 0, Math.PI * 2);
			ctx.fillStyle = iG;
			ctx.fill();

			// Iris fine detail ring (darkens outer edge)
			ctx.beginPath();
			ctx.arc(px, py, irisR, 0, Math.PI * 2);
			ctx.strokeStyle = 'rgba(0,0,0,0.32)';
			ctx.lineWidth = 5;
			ctx.stroke();

			// Iris ray texture (subtle spokes)
			ctx.save();
			ctx.globalAlpha = 0.12;
			for (let i = 0; i < 12; i++) {
				const a = (i / 12) * Math.PI * 2;
				ctx.beginPath();
				ctx.moveTo(px + Math.cos(a) * 8, py + Math.sin(a) * 8);
				ctx.lineTo(px + Math.cos(a) * irisR * 0.92, py + Math.sin(a) * irisR * 0.92);
				ctx.strokeStyle = 'rgba(180,200,255,1)';
				ctx.lineWidth = 1;
				ctx.stroke();
			}
			ctx.restore();

			// Pupil — deep gradient
			const pR = 30 * irisScale;
			const pG = ctx.createRadialGradient(px - 5, py - 6, 2, px, py, pR);
			pG.addColorStop(0, '#1c1420');
			pG.addColorStop(1, '#030204');
			ctx.beginPath();
			ctx.arc(px, py, pR, 0, Math.PI * 2);
			ctx.fillStyle = pG;
			ctx.fill();

			// Shine 1 — main bright spot
			ctx.beginPath();
			ctx.arc(px + 13, py - 15, 12, 0, Math.PI * 2);
			ctx.fillStyle = 'rgba(255,255,255,0.94)';
			ctx.fill();

			// Shine 2 — secondary (small)
			ctx.beginPath();
			ctx.arc(px + 22, py - 4, 5.5, 0, Math.PI * 2);
			ctx.fillStyle = 'rgba(255,255,255,0.60)';
			ctx.fill();

			// Ambient iris reflection — cool blue at bottom
			const ambG = ctx.createRadialGradient(
				px,
				py + irisR * 0.65,
				0,
				px,
				py + irisR * 0.65,
				irisR * 0.55
			);
			ambG.addColorStop(0, 'rgba(175, 210, 255, 0.28)');
			ambG.addColorStop(1, 'rgba(175, 210, 255, 0)');
			ctx.beginPath();
			ctx.arc(px, py, irisR, 0, Math.PI * 2);
			ctx.fillStyle = ambG;
			ctx.fill();

			// 5. Upper eyelid — dark rect clips to eye ellipse naturally
			if (blinkT > 0.01) {
				const lidH = blinkT * (ry * 2 + 10);
				ctx.fillStyle = '#0d0b1c';
				ctx.fillRect(cx - R - 5, cy - ry - 5, (R + 5) * 2, lidH + 5);
			}

			ctx.restore(); // end ellipse clip

			// 6. Eye border — crisp outline
			ctx.beginPath();
			ctx.ellipse(cx, cy, R, ry, 0, 0, Math.PI * 2);
			ctx.strokeStyle = '#100d1f';
			ctx.lineWidth = 4.5;
			ctx.stroke();

			// 7. Thick upper lash line (top 70% arc — the "eyeliner")
			ctx.beginPath();
			ctx.ellipse(cx, cy, R + 1.5, ry + 1, 0, Math.PI * 1.08, Math.PI * 1.92, false);
			ctx.strokeStyle = '#0e0b1e';
			ctx.lineWidth = 12;
			ctx.lineCap = 'round';
			ctx.stroke();

			// 8. Individual lashes — curled outward
			const LASH_N = 9;
			for (let i = 0; i < LASH_N; i++) {
				const a = Math.PI * (1.12 + (i / (LASH_N - 1)) * 0.76);
				const x1 = cx + Math.cos(a) * (R + 1);
				const y1 = cy + Math.sin(a) * (ry + 1);
				const ext = 13 + Math.sin(i * 2.1) * 4;
				// Outward curl — lashes at edges curl away more
				const curl = (i / (LASH_N - 1) - 0.5) * 0.5;
				const x2 = cx + Math.cos(a + curl) * (R + ext);
				const y2 = cy + Math.sin(a + curl) * (ry + ext * 0.55);
				ctx.beginPath();
				ctx.moveTo(x1, y1);
				ctx.lineTo(x2, y2);
				ctx.strokeStyle = '#0e0b1e';
				ctx.lineWidth = 2.5;
				ctx.lineCap = 'round';
				ctx.stroke();
			}

			// 9. Eyebrow
			const brY = cy - ry - 32 + browLift;
			// Left eye: positive tilt tilts inner-corner up (angry)
			// Right eye: mirrored so both create matching expression
			const btilt = browTilt * (side === 'left' ? 1 : -1);

			ctx.save();
			ctx.translate(cx, brY);
			ctx.rotate(btilt);

			// Brow drop shadow
			ctx.beginPath();
			ctx.moveTo(-R * 0.7, 6);
			ctx.quadraticCurveTo(R * 0.06, -9, R * 0.7, 15);
			ctx.strokeStyle = 'rgba(0,0,0,0.30)';
			ctx.lineWidth = 22;
			ctx.lineCap = 'round';
			ctx.stroke();

						// Brow fill — lighter purple-brown
			ctx.beginPath();
			ctx.moveTo(-R * 0.7, 6);
			ctx.quadraticCurveTo(R * 0.06, -9, R * 0.7, 15);
			ctx.strokeStyle = '#4a3d60';
			ctx.lineWidth = 15;
			ctx.lineCap = 'round';
			ctx.stroke();

			ctx.restore();
		}

		// ── Main animation loop ──────────────────────────
		let rafId: number;

		function tick(ts: number) {
			const dt = Math.min(lastTs ? ts - lastTs : 16, 50);
			lastTs = ts;
			elapsed += dt;

			// Timer countdowns
			nextLookMs -= dt;
			nextBlinkMs -= dt;
			nextExprMs -= dt;
			if (nextLookMs <= 0) newLookTarget();
			if (nextBlinkMs <= 0) {
				triggerBlink();
				nextBlinkMs = 2200 + Math.random() * 3600;
			}
			if (nextExprMs <= 0) newExpression();

			// ── Blink state machine ─────────────────────
			if (blinking) {
				blinkElapsed += dt;
				const close = blinkSleepy ? 200 : B_CLOSE;
				const hold = blinkSleepy ? 180 : B_HOLD;
				const open = blinkSleepy ? 280 : B_OPEN;
				let t: number;
				if (blinkElapsed < close) {
					t = blinkElapsed / close;
				} else if (blinkElapsed < close + hold) {
					t = 1;
				} else if (blinkElapsed < close + hold + open) {
					t = 1 - (blinkElapsed - close - hold) / open;
				} else {
					t = 0;
					blinking = false;
					blinkElapsed = 0;
				}
				cur.blinkL = t;
				cur.blinkR = t;
			}

			// ── Tiny organic micro-drift ────────────────
			const microX = Math.sin(elapsed * 0.00062) * 0.038;
			const microY = Math.cos(elapsed * 0.00079) * 0.028;

			// ── Lerp toward targets ─────────────────────
			const LK = lf(58, dt); // look  — snappy saccade feel
			const EX = lf(720, dt); // expression — slow & natural

			cur.lookX = lerp(cur.lookX, tgt.lookX + microX, LK);
			cur.lookY = lerp(cur.lookY, tgt.lookY + microY, LK);
			cur.browLiftL = lerp(cur.browLiftL, tgt.browLiftL, EX);
			cur.browLiftR = lerp(cur.browLiftR, tgt.browLiftR, EX);
			cur.browTiltL = lerp(cur.browTiltL, tgt.browTiltL, EX);
			cur.browTiltR = lerp(cur.browTiltR, tgt.browTiltR, EX);
			cur.irisScale = lerp(cur.irisScale, tgt.irisScale, EX);

			// ── Render ──────────────────────────────────
			ctx.clearRect(0, 0, w, h);

			drawEye(
				L.cx,
				L.cy,
				cur.lookX,
				cur.lookY,
				cur.blinkL,
				cur.browLiftL,
				cur.browTiltL,
				cur.irisScale,
				'left'
			);
			drawEye(
				Ri.cx,
				Ri.cy,
				cur.lookX,
				cur.lookY,
				cur.blinkR,
				cur.browLiftR,
				cur.browTiltR,
				cur.irisScale,
				'right'
			);

			rafId = requestAnimationFrame(tick);
		}

		rafId = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(rafId);
	});
</script>

<canvas bind:this={canvas} style="position:fixed;inset:0;z-index:9999;pointer-events:none;"></canvas>
