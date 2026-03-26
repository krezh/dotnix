package templates

import (
	"fmt"
	"strings"

	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
)

func graphHistoryJSONf64(history []float64) string {
	if len(history) == 0 {
		return "[]"
	}
	var sb strings.Builder
	sb.WriteByte('[')
	for i, v := range history {
		if i > 0 {
			sb.WriteByte(',')
		}
		sb.WriteString(fmt.Sprintf("%.4f", v))
	}
	sb.WriteByte(']')
	return sb.String()
}

// netSparkHistoryJSON serialises a uint64 history slice as a compact JSON array string.
func netSparkHistoryJSON(history []uint64) string {
	if len(history) == 0 {
		return "[]"
	}
	var sb strings.Builder
	sb.WriteByte('[')
	for i, v := range history {
		if i > 0 {
			sb.WriteByte(',')
		}
		sb.WriteString(fmt.Sprintf("%d", v))
	}
	sb.WriteByte(']')
	return sb.String()
}

// graphYs computes the SVG y-coordinate array for a right-aligned graph.
// y=0 is the top, y=h is the baseline. Data is right-aligned; empty slots stay at h.
// fixedMax > 0 forces a fixed scale; 0 means auto-scale to the slice max.
func graphYs(vals []float64, totalPts, h int, fixedMax float64) []float64 {
	maxVal := fixedMax
	if maxVal == 0 {
		for _, v := range vals {
			if v > maxVal {
				maxVal = v
			}
		}
	}
	ys := make([]float64, totalPts)
	offset := totalPts - len(vals)
	for i := 0; i < totalPts; i++ {
		histIdx := i - offset
		if histIdx < 0 || maxVal == 0 {
			ys[i] = float64(h)
		} else {
			ys[i] = float64(h) - (vals[histIdx]/maxVal)*float64(h-2)
		}
	}
	return ys
}

// graphLinePath generates the SVG path d attribute for a smooth line through all points.
func graphLinePath(vals []float64, totalPts, h int, fixedMax float64) string {
	ys := graphYs(vals, totalPts, h, fixedMax)
	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("M 0,%.2f", ys[0]))
	for i := 1; i < totalPts; i++ {
		cpx := (float64(i-1) + float64(i)) / 2
		sb.WriteString(fmt.Sprintf(" C %.2f,%.2f %.2f,%.2f %.2f,%.2f", cpx, ys[i-1], cpx, ys[i], float64(i), ys[i]))
	}
	return sb.String()
}

// graphFillPath generates the SVG path d attribute for the filled area under the line.
func graphFillPath(vals []float64, totalPts, h int, fixedMax float64) string {
	line := graphLinePath(vals, totalPts, h, fixedMax)
	return fmt.Sprintf("%s L %.2f,%d L 0,%d Z", line, float64(totalPts-1), h, h)
}

// netSparkMaxVal returns the maximum value in the history slice.
func netSparkMaxVal(history []uint64) uint64 {
	var maxVal uint64
	for _, v := range history {
		if v > maxVal {
			maxVal = v
		}
	}
	return maxVal
}

// netSparkMaxLabel returns a formatted label for the peak value in the history slice.
func netSparkMaxLabel(history []uint64) string {
	maxVal := netSparkMaxVal(history)
	if maxVal == 0 {
		return "0 B/s"
	}
	return system.FormatBytes(maxVal) + "/s"
}

// u64ToF64 converts a []uint64 to []float64.
func u64ToF64(history []uint64) []float64 {
	out := make([]float64, len(history))
	for i, v := range history {
		out[i] = float64(v)
	}
	return out
}

// netSparkLinePath generates the SVG line path for a network sparkline (auto-scale).
func netSparkLinePath(history []uint64, totalPts, h int) string {
	return graphLinePath(u64ToF64(history), totalPts, h, 0)
}

// netSparkFillPath generates the SVG fill path for a network sparkline (auto-scale).
func netSparkFillPath(history []uint64, totalPts, h int) string {
	return graphFillPath(u64ToF64(history), totalPts, h, 0)
}

// netIfaceIcon renders an SVG icon representing the interface type.

// Dashboard renders the dashboard page.
func Dashboard(info *system.SystemInfo, cpu *system.CPUStats, cpuHistory []float64, mem *system.MemoryStats, disks []system.DiskStats, load []float64, gpus []system.GPUStats, network []system.NetworkStats, temps *system.TemperatureStats, procs []system.ProcessInfo) g.Node {
	return Base("Dashboard", DashboardContent(info, cpu, cpuHistory, mem, disks, load, gpus, network, temps, procs))
}

// DashboardContent renders the dashboard content with system info and stats grid.
func DashboardContent(info *system.SystemInfo, cpu *system.CPUStats, cpuHistory []float64, mem *system.MemoryStats, disks []system.DiskStats, load []float64, gpus []system.GPUStats, network []system.NetworkStats, temps *system.TemperatureStats, procs []system.ProcessInfo) g.Node {
	return g.Group([]g.Node{
		// System Info
		html.Div(
			html.Class("grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8"),
			html.Div(
				html.Class("stat-card"),
				html.Div(html.Class("stat-label"), g.Text("Hostname")),
				html.Div(html.Class("stat-value"), g.Text(info.Hostname)),
			),
			html.Div(
				html.Class("stat-card"),
				html.Div(html.Class("stat-label"), g.Text("Uptime")),
				html.Div(html.Class("stat-value"), g.Text(system.FormatUptime(info.Uptime))),
			),
			html.Div(
				html.Class("stat-card"),
				html.Div(html.Class("stat-label"), g.Text("NixOS Version")),
				html.Div(html.Class("stat-value break-all"), g.Text(info.NixOSVersion)),
			),
			html.Div(
				html.Class("stat-card"),
				html.Div(html.Class("stat-label"), g.Text("Kernel")),
				html.Div(html.Class("stat-value"), g.Text(info.KernelVersion)),
			),
		),
		// Stats Grid
		html.Div(
			html.ID("stats-container"),
			html.Class("grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8 min-h-[600px]"),
			StatsGrid(cpu, cpuHistory, mem, disks, load, gpus, network, temps, procs),
		),
		html.Script(
			html.Type("text/javascript"),
			g.Raw(`
		(function() {
			const statsContainer = document.getElementById('stats-container');

			// --- Process sort state ---
			let procSortCol = 'cpu'; // default: sort by CPU descending
			let procSortDir = -1;    // -1 = descending, 1 = ascending

			// Switch active sort column (or toggle direction if same column).
			window.procSort = function(col) {
				if (procSortCol === col) {
					procSortDir *= -1;
				} else {
					procSortCol = col;
					// Numeric columns default descending; name/pid default ascending.
					procSortDir = (col === 'name' || col === 'pid') ? 1 : -1;
				}
				applyProcSort();
			};

			// Re-sort the process tbody rows and update arrow indicators.
			function applyProcSort() {
				const tbody = document.getElementById('proc-tbody');
				if (!tbody) return;

				const rows = Array.from(tbody.querySelectorAll('tr'));

				rows.sort(function(a, b) {
					let primary;
					if (procSortCol === 'name') {
						const valA = (a.cells[1] ? a.cells[1].textContent.trim().toLowerCase() : '');
						const valB = (b.cells[1] ? b.cells[1].textContent.trim().toLowerCase() : '');
						primary = procSortDir * valA.localeCompare(valB);
					} else {
						const valA = parseFloat(a.dataset[procSortCol] ?? 0);
						const valB = parseFloat(b.dataset[procSortCol] ?? 0);
						primary = procSortDir * (valA - valB);
					}
					// Break ties by PID so order is fully deterministic.
					if (primary !== 0) return primary;
					return parseInt(a.dataset.pid) - parseInt(b.dataset.pid);
				});

				rows.forEach(row => tbody.appendChild(row));
				updateProcArrows();
			}

			// Update the sort arrow indicators in column headers.
			function updateProcArrows() {
				const cols = ['pid', 'name', 'cpu', 'mem', 'mempct'];
				cols.forEach(col => {
					const el = document.getElementById('proc-arrow-' + col);
					if (!el) return;
					if (col === procSortCol) {
						el.textContent = procSortDir === -1 ? '↓' : '↑';
						el.classList.remove('text-gray-400');
						el.classList.add('text-blue-500');
					} else {
						el.textContent = '';
						el.classList.remove('text-blue-500');
						el.classList.add('text-gray-400');
					}
				});
			}

			// --- Progress bar colors ---
			function getProgressColor(percent) {
				const isDark = document.documentElement.classList.contains('dark');
				const p = parseFloat(percent);
				if (p >= 80) return isDark ? '#ef4444' : '#dc2626';
				if (p >= 61) return isDark ? '#facc15' : '#eab308';
				return isDark ? '#3b82f6' : '#2563eb';
			}

			function applyProgressColors() {
				document.querySelectorAll('.progress-fill-gradient[data-percent]').forEach(bar => {
					bar.style.backgroundColor = getProgressColor(bar.dataset.percent);
				});
			}

			// Morphing function to update DOM smoothly without destroying elements.
			function morphUpdate(newHtml) {
				const tempDiv = document.createElement('div');
				tempDiv.innerHTML = newHtml;

				// Update progress bars.
				tempDiv.querySelectorAll('.progress-fill-gradient[id], .progress-fill[id]').forEach(newBar => {
					const oldBar = document.getElementById(newBar.id);
					if (oldBar) {
						oldBar.style.width = newBar.style.width;
						if (newBar.dataset.percent) oldBar.dataset.percent = newBar.dataset.percent;
					}
				});

				// Update text content for all elements with IDs (skip proc card and graph containers).
				// Graph containers are owned by the JS graph animator — never touch their innerHTML.
				tempDiv.querySelectorAll('[id]').forEach(newEl => {
					if (newEl.id === 'proc-card' || newEl.id === 'proc-tbody' || newEl.id.startsWith('proc-arrow-')) return;
				if (newEl.id === 'cpu-history') return;
				if (newEl.id.endsWith('-rx-spark') || newEl.id.endsWith('-tx-spark')) return;
				if (newEl.id === 'mem-spark' || newEl.id === 'swap-spark') return;
				if (newEl.id.endsWith('-util-spark') || newEl.id.endsWith('-mem-spark')) return;
				if (newEl.id.endsWith('-temp-spark')) return;
					const oldEl = document.getElementById(newEl.id);
					if (!oldEl) return;
					if (oldEl.textContent !== newEl.textContent) oldEl.textContent = newEl.textContent;
					if (newEl.dataset.percent !== undefined) oldEl.dataset.percent = newEl.dataset.percent;
				});

				// Keyed reconcile of proc tbody rows by PID — no full rebuild.
				const newTbody = tempDiv.querySelector('#proc-tbody');
				const oldTbody = document.getElementById('proc-tbody');
				let procSetChanged = false;
				if (newTbody && oldTbody) {
					const newRows = Array.from(newTbody.querySelectorAll('tr'));
					const newByPid = new Map(newRows.map(r => [r.dataset.pid, r]));

					// Update or remove existing rows.
					Array.from(oldTbody.querySelectorAll('tr')).forEach(oldRow => {
						const pid = oldRow.dataset.pid;
						const newRow = newByPid.get(pid);
						if (!newRow) {
							oldRow.remove();
							procSetChanged = true;
							return;
						}
						// Update data attributes.
						oldRow.dataset.cpu    = newRow.dataset.cpu;
						oldRow.dataset.mem    = newRow.dataset.mem;
						oldRow.dataset.mempct = newRow.dataset.mempct;
						// Update cell text only when changed.
						Array.from(newRow.cells).forEach((newCell, i) => {
							if (oldRow.cells[i] && oldRow.cells[i].textContent !== newCell.textContent) {
								oldRow.cells[i].textContent = newCell.textContent;
							}
						});
						newByPid.delete(pid); // mark as handled
					});

					// Append genuinely new rows.
					newByPid.forEach(newRow => {
						oldTbody.appendChild(newRow);
						procSetChanged = true;
					});
				}

				applyProgressColors();
				// Only re-sort when the set of PIDs changed, or when sorted by a
				// live column (cpu/mem/mempct) where values meaningfully change.
				const stableCol = (procSortCol === 'pid' || procSortCol === 'name');
				if (procSetChanged || !stableCol) {
					applyProcSort();
				}
			}

			// Apply initial state.
			applyProgressColors();
			applyProcSort();

			// Connect to SSE stream (server controls polling rate).
			const eventSource = new EventSource('/api/stats/stream');
			window._dashboardSSE = eventSource;

			eventSource.onopen = function() { console.log('Stats stream connected'); };
			eventSource.onmessage = function(event) { morphUpdate(event.data); };
			eventSource.onerror = function(error) {
				console.error('Stats stream error:', error);
				statsContainer.innerHTML = '<div class="card lg:col-span-2 flex items-center justify-center py-12">' +
					'<div class="text-red-600">Failed to load stats. Refresh the page to try again.</div>' +
				'</div>';
				eventSource.close();
			};

			// --- Smooth graph animator (CPU + network) ---
			// Graphs declare themselves via data attributes on their container div:
			//   data-history  JSON array of historical values (seeds the ring buffer)
			//   data-color    stroke/fill colour
			//   data-scale    "auto" (default) = scale to buffer max | number = fixed max value
			const GRAPH_POINTS = 90;   // visible data points
			const GRAPH_W      = 89;   // SVG viewBox width (GRAPH_POINTS - 1 segments)
			const GRAPH_H      = 40;   // SVG viewBox height
			const TICK_MS      = 2000; // server poll interval

			// graphState[id] = { buf, color, fixedMax, runningMax, fillPath, linePath, lastPush, rafId }
			const graphState = {};
			window._graphState = graphState;

			// --- Shared tooltip element ---
			const graphTooltip = document.createElement('span');
			graphTooltip.className = 'graph-tooltip';
			document.body.appendChild(graphTooltip);
			window._graphTooltip = graphTooltip;

			function showGraphTooltip(container, clientX, clientY) {
				const state = graphState[container.id];
				if (!state) return;
				const rect = container.getBoundingClientRect();
				const relX = clientX - rect.left;
				// Map pixel X to buffer slot. The graph scrolls: the rightmost slot is
				// at the right edge, oldest at the left. Account for current scroll offset.
				const elapsed = performance.now() - state.lastPush;
				const t = Math.min(elapsed / TICK_MS, 1);
				const offsetX = t * (GRAPH_W / (GRAPH_POINTS - 1));
				// Slot index: pixel maps to SVG x, corrected for scroll.
				const svgX = (relX / rect.width) * GRAPH_W + offsetX;
				const slotF = svgX * (GRAPH_POINTS - 1) / GRAPH_W;
				const slot = Math.round(Math.max(0, Math.min(state.buf.length - 1, slotF)));
				const value = state.buf[slot];
				// Format value based on graph type.
				let label;
				if (state.tempGraph) {
					label = value.toFixed(1) + '°C';
				} else if (state.fixedMax === 100) {
					label = value.toFixed(1) + '%';
				} else {
					label = formatBytes(value) + '/s';
				}
				graphTooltip.textContent = label;
				graphTooltip.classList.remove('invisible', 'opacity-0');
				graphTooltip.classList.add('visible', 'opacity-100');
				// Position above cursor, clamped to viewport.
				// Make visible first so offsetWidth/offsetHeight are accurate.
				const tw = graphTooltip.offsetWidth;
				const th = graphTooltip.offsetHeight;
				const tx = Math.min(clientX - tw / 2, window.innerWidth - tw - 4);
				const ty = clientY - th - 10;
				graphTooltip.style.left = Math.max(4, tx) + 'px';
				graphTooltip.style.top = (ty < 0 ? clientY + 14 : ty) + 'px';
			}

			function hideGraphTooltip() {
				graphTooltip.classList.add('invisible', 'opacity-0');
				graphTooltip.classList.remove('visible', 'opacity-100');
			}

			function buildSVGPath(state, offsetX) {
				const buf = state.buf;
				const count = buf.length;
				// Use the pre-computed running max; fall back to fixedMax.
				let max = state.fixedMax > 0 ? state.fixedMax : state.runningMax;

				const ys = new Float64Array(count);
				for (let i = 0; i < count; i++) {
					ys[i] = max === 0 ? GRAPH_H : GRAPH_H - (buf[i] / max) * (GRAPH_H - 2);
				}

				// Build path string with array + join to avoid repeated string concatenation.
				const segs = ['M ' + (0 - offsetX).toFixed(2) + ',' + ys[0].toFixed(2)];
				for (let i = 1; i < count; i++) {
					const cpx = (i - 1 + i) / 2 - offsetX;
					segs.push(' C ' + cpx.toFixed(2) + ',' + ys[i-1].toFixed(2) +
					           ' '  + cpx.toFixed(2) + ',' + ys[i].toFixed(2) +
					           ' '  + (i - offsetX).toFixed(2) + ',' + ys[i].toFixed(2));
				}
				const line = segs.join('');
				const fill = line + ' L ' + (count - 1 - offsetX).toFixed(2) + ',' + GRAPH_H +
				             ' L ' + (0 - offsetX).toFixed(2) + ',' + GRAPH_H + ' Z';
				return { line, fill };
			}

			// Interpolates a temperature value to a hex color.
			// <60°C: blue; 60–79°C: blue → yellow; ≥80°C: yellow → red.
			function tempColor(celsius) {
				function lerpChannel(a, b, t) { return Math.round(a + (b - a) * t); }
				function toHex(r, g, b) {
					return '#' + [r, g, b].map(function(v) {
						return ('0' + v.toString(16)).slice(-2);
					}).join('');
				}
				// blue   #3b82f6 = rgb(59,130,246)
				// yellow #eab308 = rgb(234,179,8)
				// red    #ef4444 = rgb(239,68,68)
				if (celsius < 60) return '#3b82f6';
				if (celsius < 80) {
					const t = (celsius - 60) / 20;
					return toHex(lerpChannel(59,234,t), lerpChannel(130,179,t), lerpChannel(246,8,t));
				}
				const t = Math.min((celsius - 80) / 20, 1);
				return toHex(lerpChannel(234,239,t), lerpChannel(179,68,t), lerpChannel(8,68,t));
			}

			function renderGraph(id) {
				const state = graphState[id];
				if (!state) return;

				// Continuously scroll: offsetX advances linearly at 1 slot per TICK_MS.
				const elapsed = performance.now() - state.lastPush;
				const t = Math.min(elapsed / TICK_MS, 1);
				const offsetX = t * (GRAPH_W / (GRAPH_POINTS - 1));

				// For temperature graphs, update the linearGradient stops per-slot.
				if (state.tempGraph && state.gradStops) {
					const n = state.buf.length;
					const slotW = GRAPH_W / (GRAPH_POINTS - 1);
					for (let i = 0; i < n; i++) {
						const svgX = i * slotW - offsetX;
						const pct = (svgX / GRAPH_W * 100).toFixed(2) + '%';
						const col = tempColor(state.buf[i]);
						const stop = state.gradStops[i];
						if (stop.getAttribute('offset') !== pct) stop.setAttribute('offset', pct);
						if (stop.getAttribute('stop-color') !== col) stop.setAttribute('stop-color', col);
					}
				}

				const paths = buildSVGPath(state, offsetX);
				// Use cached path element references — no querySelector per frame.
				state.fillPath.setAttribute('d', paths.fill);
				state.linePath.setAttribute('d', paths.line);

				state.rafId = requestAnimationFrame(() => renderGraph(id));
			}

			function initGraph(container) {
				const id = container.id;
				const tempGraph = container.dataset.tempgraph === '1';
				const scaleAttr = container.dataset.scale;
				const fixedMax = (scaleAttr && scaleAttr !== 'auto') ? parseFloat(scaleAttr) : 0;

				let history = [];
				try { history = JSON.parse(container.dataset.history || '[]'); } catch(e) {}

				// Ring buffer: GRAPH_POINTS + 1 (extra slot visible during scroll).
				const buf = new Float64Array(GRAPH_POINTS + 1);
				const start = Math.max(0, GRAPH_POINTS + 1 - history.length);
				for (let i = start; i < GRAPH_POINTS + 1; i++) {
					buf[i] = history[i - start] || 0;
				}

				// Compute initial running max from seeded history.
				let runningMax = 0;
				for (let i = 0; i < buf.length; i++) if (buf[i] > runningMax) runningMax = buf[i];

				const color = container.dataset.color || '#3b82f6';

				// Ensure SVG has fill + line path elements.
				let svg = container.querySelector('svg');
				let fillPath, linePath;
				if (!svg) {
					svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
					svg.setAttribute('class', 'w-full h-full');
					svg.setAttribute('viewBox', '0 0 ' + GRAPH_W + ' ' + GRAPH_H);
					svg.setAttribute('preserveAspectRatio', 'none');
					fillPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
					fillPath.setAttribute('fill', color);
					fillPath.setAttribute('fill-opacity', '0.15');
					fillPath.setAttribute('stroke', 'none');
					linePath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
					linePath.setAttribute('fill', 'none');
					linePath.setAttribute('stroke', color);
					linePath.setAttribute('stroke-width', '1.5');
					linePath.setAttribute('stroke-linecap', 'round');
					linePath.setAttribute('stroke-linejoin', 'round');
					linePath.setAttribute('vector-effect', 'non-scaling-stroke');
					svg.appendChild(fillPath);
					svg.appendChild(linePath);
					container.innerHTML = '';
					container.appendChild(svg);
				} else {
					// SVG already exists (server-rendered); cache its path elements.
					const pathEls = svg.querySelectorAll('path');
					fillPath = pathEls[0];
					linePath = pathEls[1];
				}

				// For temperature graphs, inject a linearGradient and point paths at it.
				let gradStops = null;
				if (tempGraph) {
					const NS = 'http://www.w3.org/2000/svg';
					const gradId = 'temp-grad-' + id;
					const defs = document.createElementNS(NS, 'defs');
					const grad = document.createElementNS(NS, 'linearGradient');
					grad.setAttribute('id', gradId);
					grad.setAttribute('x1', '0%');
					grad.setAttribute('x2', '100%');
					grad.setAttribute('y1', '0%');
					grad.setAttribute('y2', '0%');
					grad.setAttribute('gradientUnits', 'userSpaceOnUse');
					grad.setAttribute('x1', '0');
					grad.setAttribute('x2', String(GRAPH_W));
					gradStops = [];
					for (let i = 0; i < buf.length; i++) {
						const stop = document.createElementNS(NS, 'stop');
						stop.setAttribute('offset', '0%');
						stop.setAttribute('stop-color', tempColor(buf[i]));
						grad.appendChild(stop);
						gradStops.push(stop);
					}
					defs.appendChild(grad);
					svg.insertBefore(defs, svg.firstChild);
					const ref = 'url(#' + gradId + ')';
					fillPath.setAttribute('fill', ref);
					fillPath.setAttribute('fill-opacity', '0.15');
					linePath.setAttribute('stroke', ref);
				}

				// Store cached DOM refs alongside graph state.
				graphState[id] = {
					buf, color, fixedMax, runningMax,
					fillPath, linePath, tempGraph, gradStops,
					lastPush: performance.now() - TICK_MS, rafId: null
				};
				renderGraph(id);

				container.addEventListener('mousemove', function(e) {
					showGraphTooltip(container, e.clientX, e.clientY);
				});
				container.addEventListener('mouseleave', hideGraphTooltip);
			}

			function pushGraphPoint(id, value) {
				const state = graphState[id];
				if (!state) return;

				// Track the value being evicted (leftmost slot) for incremental max.
				const evicted = state.buf[0];

				// Shift buffer left and append new value.
				state.buf.copyWithin(0, 1);
				state.buf[state.buf.length - 1] = value;

				// Update running max incrementally.
				if (state.fixedMax === 0) {
					if (value >= state.runningMax) {
						// New value is at least as large as current max — no scan needed.
						state.runningMax = value;
					} else if (evicted >= state.runningMax) {
						// The evicted value was the max — rescan entire buffer.
						let max = 0;
						for (let i = 0; i < state.buf.length; i++) if (state.buf[i] > max) max = state.buf[i];
						state.runningMax = max;
					}
					// Otherwise runningMax is still valid.
				}

				// Reset scroll phase — the new point is now at the right edge.
				state.lastPush = performance.now();
			}

			function updateNetMaxLabel(ifaceIdx, rxBuf, txBuf) {
				let rxMax = 0, txMax = 0;
				for (let i = 0; i < rxBuf.length; i++) if (rxBuf[i] > rxMax) rxMax = rxBuf[i];
				for (let i = 0; i < txBuf.length; i++) if (txBuf[i] > txMax) txMax = txBuf[i];
				const rxEl = document.getElementById('net' + ifaceIdx + '-rx-maxlabel');
				const txEl = document.getElementById('net' + ifaceIdx + '-tx-maxlabel');
				if (rxEl) rxEl.textContent = formatBytes(rxMax) + '/s';
				if (txEl) txEl.textContent = formatBytes(txMax) + '/s';
			}

			function formatBytes(bytes) {
				if (bytes < 1024) return bytes + ' B';
				const units = ['KiB','MiB','GiB','TiB'];
				let v = bytes, u = -1;
				do { v /= 1024; u++; } while (v >= 1024 && u < units.length - 1);
				return v.toFixed(1) + ' ' + units[u];
			}

			// Initialise all graphs from their server-rendered data-history attributes.
			document.querySelectorAll('#cpu-history, #mem-spark, #swap-spark, [id$="-rx-spark"], [id$="-tx-spark"], [id$="-util-spark"], [id$="-mem-spark"], [id$="-temp-spark"]').forEach(initGraph);

			// Network data points — push to ring buffers and update max labels.
			eventSource.addEventListener('netdata', function(event) {
				let points;
				try { points = JSON.parse(event.data); } catch(e) { return; }
				points.forEach(function(p) {
					pushGraphPoint('net' + p.i + '-rx-spark', p.rx);
					pushGraphPoint('net' + p.i + '-tx-spark', p.tx);
					const rxState = graphState['net' + p.i + '-rx-spark'];
					const txState = graphState['net' + p.i + '-tx-spark'];
					if (rxState && txState) updateNetMaxLabel(p.i, rxState.buf, txState.buf);
				});
			});

			// CPU data point — push to ring buffer (fixed 0–100 scale, no label update needed).
			eventSource.addEventListener('cpudata', function(event) {
				pushGraphPoint('cpu-history', parseFloat(event.data));
			});

			// Memory data points — push RAM and swap percent to their ring buffers.
			eventSource.addEventListener('memdata', function(event) {
				let point;
				try { point = JSON.parse(event.data); } catch(e) { return; }
				pushGraphPoint('mem-spark', point.ram);
				if (point.swap > 0) pushGraphPoint('swap-spark', point.swap);
			});

			// Temperature data points — push CPU and GPU temps to their ring buffers.
			eventSource.addEventListener('tempdata', function(event) {
				let point;
				try { point = JSON.parse(event.data); } catch(e) { return; }
				if (point.cpu > 0) pushGraphPoint('cpu-temp-spark', point.cpu);
				if (point.gpus) point.gpus.forEach(function(g) {
					pushGraphPoint('gpu' + g.i + '-temp-spark', g.temp);
				});
			});

			// GPU data points — push utilisation and VRAM percent to their ring buffers.
			eventSource.addEventListener('gpudata', function(event) {
				let points;
				try { points = JSON.parse(event.data); } catch(e) { return; }
				points.forEach(function(p) {
					pushGraphPoint('gpu' + p.i + '-util-spark', p.util);
					pushGraphPoint('gpu' + p.i + '-mem-spark', p.mem);
				});
			});

			window.addEventListener('beforeunload', function() { eventSource.close(); });

			// Smooth expand/collapse animation for <details> elements.
			// Only the panel below <summary> slides; the summary row itself stays fixed.
			document.querySelectorAll('details').forEach(function(details) {
				// Target the first element sibling after <summary> — the content panel.
				const summary = details.querySelector('summary');
				const panel = summary ? summary.nextElementSibling : null;
				if (!panel) return;
				details.addEventListener('click', function(e) {
					if (!e.target.closest('summary')) return;
					e.preventDefault();
					if (details.open) {
						// Collapse: animate panel height to 0, then close.
						details.style.overflow = 'hidden';
						panel.animate(
							[{ height: panel.offsetHeight + 'px' }, { height: '0px' }],
							{ duration: 200, easing: 'ease-in-out' }
						).onfinish = function() {
							details.open = false;
							details.style.overflow = '';
						};
					} else {
						// Expand: open so panel is rendered, measure, animate from 0.
						details.open = true;
						details.style.overflow = 'hidden';
						const h = panel.offsetHeight;
						panel.animate(
							[{ height: '0px' }, { height: h + 'px' }],
							{ duration: 200, easing: 'ease-in-out' }
						).onfinish = function() {
							details.style.overflow = '';
						};
					}
				});
			});
		})();
			`),
		),
	})
}

// netIfaceIcon renders an SVG icon representing the network interface type.
func netIfaceIcon(ifType system.NetworkInterfaceType) g.Node {
	switch ifType {
	case system.NetTypeEthernet:
		return g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
		</svg>`)
	case system.NetTypeWifi:
		return g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"></path>
		</svg>`)
	case system.NetTypeOther:
		return g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
		</svg>`)
	default:
		return g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
		</svg>`)
	}
}

// StatsGrid renders the complete stats grid with CPU, Memory, Network, GPU, Disks, and Processes.
func StatsGrid(cpu *system.CPUStats, cpuHistory []float64, mem *system.MemoryStats, disks []system.DiskStats, load []float64, gpus []system.GPUStats, network []system.NetworkStats, temps *system.TemperatureStats, procs []system.ProcessInfo) g.Node {
	return g.Group([]g.Node{
		// CPU Stats
		html.Div(
			html.Class("card"),
			html.H3(html.Class("text-base font-semibold mb-3"), g.Text("CPU")),
			html.Div(
				html.Class("mb-3"),
				html.Div(
					html.Class("flex justify-between mb-2"),
					html.Span(
						html.Class("text-sm font-medium"),
						g.Text("Current: "),
						html.Span(html.ID("cpu-percent"), g.Textf("%.1f%%", cpu.UsagePercent)),
					),
					html.Span(html.Class("text-xs text-gray-500 dark:text-gray-400"), g.Text("Last 3 minutes")),
				),
				// CPU History Graph
				html.Div(
					html.ID("cpu-history"),
					html.Class("h-24 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
					DataAttr("history", graphHistoryJSONf64(cpuHistory)),
					DataAttr("color", "#0284c7"),
					DataAttr("scale", "100"),
					g.El("svg",
						html.Class("w-full h-full"),
						g.Attr("viewBox", "0 0 89 40"),
						g.Attr("preserveAspectRatio", "none"),
						g.El("path",
							g.Attr("d", graphFillPath(cpuHistory, 90, 40, 100)),
							g.Attr("fill", "#0284c7"),
							g.Attr("fill-opacity", "0.15"),
							g.Attr("stroke", "none"),
						),
						g.El("path",
							g.Attr("d", graphLinePath(cpuHistory, 90, 40, 100)),
							g.Attr("fill", "none"),
							g.Attr("stroke", "#0284c7"),
							g.Attr("stroke-width", "1.5"),
							g.Attr("stroke-linecap", "round"),
							g.Attr("stroke-linejoin", "round"),
							g.Attr("vector-effect", "non-scaling-stroke"),
						),
					),
				),
			),
			html.Div(
				html.Class("space-y-1 text-sm text-gray-600 dark:text-gray-400"),
				html.Div(
					html.Class("flex justify-between"),
					html.Span(g.Text("Model:")),
					html.Span(
						html.Class("text-right truncate ml-2"),
						html.Title(cpu.ModelName),
						g.Text(cpu.ModelName),
					),
				),
				html.Div(
					html.Class("flex justify-between"),
					html.Span(g.Text("Configuration:")),
					html.Span(g.Textf("%d threads", cpu.LogicalCores)),
				),
				g.If(len(load) >= 3,
					html.Div(
						html.Class("flex justify-between"),
						html.Span(g.Text("Load Average (1m/5m/15m):")),
						html.Span(html.ID("load-avg"), g.Textf("%.2f / %.2f / %.2f", load[0], load[1], load[2])),
					),
				),
				g.If(temps != nil && temps.CPUTemp > 0,
					g.Group([]g.Node{
						html.Div(
							html.Class("flex justify-between mb-1"),
							html.Span(g.Text("Temperature:")),
							html.Span(html.ID("cpu-temp"), g.Textf("%.0f°C", temps.CPUTemp)),
						),
						html.Div(
							html.ID("cpu-temp-spark"),
							html.Class("h-12 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
							DataAttr("history", graphHistoryJSONf64(temps.CPUTempHistory)),
							DataAttr("scale", "120"),
							DataAttr("tempgraph", "1"),
							g.El("svg",
								html.Class("w-full h-full"),
								g.Attr("viewBox", "0 0 89 40"),
								g.Attr("preserveAspectRatio", "none"),
								g.El("path",
									g.Attr("d", graphFillPath(temps.CPUTempHistory, 90, 40, 120)),
									g.Attr("fill", "#3b82f6"),
									g.Attr("fill-opacity", "0.15"),
									g.Attr("stroke", "none"),
								),
								g.El("path",
									g.Attr("d", graphLinePath(temps.CPUTempHistory, 90, 40, 120)),
									g.Attr("fill", "none"),
									g.Attr("stroke", "#3b82f6"),
									g.Attr("stroke-width", "1.5"),
									g.Attr("stroke-linecap", "round"),
									g.Attr("stroke-linejoin", "round"),
									g.Attr("vector-effect", "non-scaling-stroke"),
								),
							),
						),
					}),
				),
			),
		),

		// Memory Stats
		html.Div(
			html.Class("card"),
			html.H3(html.Class("text-base font-semibold mb-3"), g.Text("Memory")),
			html.Div(
				html.Class("mb-3"),
				html.Div(
					html.Class("flex justify-between mb-1"),
					html.Span(html.Class("text-sm font-medium"), g.Text("RAM")),
					html.Span(html.Class("text-sm font-medium"), html.ID("mem-percent"), g.Textf("%.1f%%", mem.UsedPercent)),
				),
				html.Div(
					html.ID("mem-spark"),
					html.Class("h-16 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
					DataAttr("history", graphHistoryJSONf64(mem.RAMHistory)),
					DataAttr("color", "#0284c7"),
					DataAttr("scale", "100"),
					g.El("svg",
						html.Class("w-full h-full"),
						g.Attr("viewBox", "0 0 89 40"),
						g.Attr("preserveAspectRatio", "none"),
						g.El("path",
							g.Attr("d", graphFillPath(mem.RAMHistory, 90, 40, 100)),
							g.Attr("fill", "#0284c7"),
							g.Attr("fill-opacity", "0.15"),
							g.Attr("stroke", "none"),
						),
						g.El("path",
							g.Attr("d", graphLinePath(mem.RAMHistory, 90, 40, 100)),
							g.Attr("fill", "none"),
							g.Attr("stroke", "#0284c7"),
							g.Attr("stroke-width", "1.5"),
							g.Attr("stroke-linecap", "round"),
							g.Attr("stroke-linejoin", "round"),
							g.Attr("vector-effect", "non-scaling-stroke"),
						),
					),
				),
				html.Div(
					html.Class("flex justify-between mt-1 text-xs text-gray-600 dark:text-gray-400"),
					html.Span(html.ID("mem-used"), g.Textf("%s used", system.FormatBytes(mem.Used))),
					html.Span(g.Textf("%s available", system.FormatBytes(mem.Available))),
				),
			),
			g.If(mem.SwapTotal > 0,
				html.Div(
					html.Class("mb-3"),
					html.Div(
						html.Class("flex justify-between mb-1"),
						html.Span(html.Class("text-sm font-medium"), g.Text("Swap")),
						html.Span(html.Class("text-sm font-medium"), html.ID("swap-percent"), g.Textf("%.1f%%", mem.SwapPercent)),
					),
					html.Div(
						html.ID("swap-spark"),
						html.Class("h-16 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
						DataAttr("history", graphHistoryJSONf64(mem.SwapHistory)),
						DataAttr("color", "#a855f7"),
						DataAttr("scale", "100"),
						g.El("svg",
							html.Class("w-full h-full"),
							g.Attr("viewBox", "0 0 89 40"),
							g.Attr("preserveAspectRatio", "none"),
							g.El("path",
								g.Attr("d", graphFillPath(mem.SwapHistory, 90, 40, 100)),
								g.Attr("fill", "#a855f7"),
								g.Attr("fill-opacity", "0.15"),
								g.Attr("stroke", "none"),
							),
							g.El("path",
								g.Attr("d", graphLinePath(mem.SwapHistory, 90, 40, 100)),
								g.Attr("fill", "none"),
								g.Attr("stroke", "#a855f7"),
								g.Attr("stroke-width", "1.5"),
								g.Attr("stroke-linecap", "round"),
								g.Attr("stroke-linejoin", "round"),
								g.Attr("vector-effect", "non-scaling-stroke"),
							),
						),
					),
					html.Div(
						html.Class("flex justify-between mt-1 text-xs text-gray-600 dark:text-gray-400"),
						html.Span(html.ID("swap-used"), g.Textf("%s used", system.FormatBytes(mem.SwapUsed))),
						html.Span(g.Textf("%s total", system.FormatBytes(mem.SwapTotal))),
					),
				),
			),
			html.Div(
				html.Class("space-y-1 text-sm text-gray-600 dark:text-gray-400"),
				html.Div(
					html.Class("flex justify-between"),
					html.Span(g.Text("Total RAM:")),
					html.Span(g.Text(system.FormatBytes(mem.Total))),
				),
				g.If(temps != nil && temps.MemoryTemp > 0,
					html.Div(
						html.Class("flex justify-between"),
						html.Span(g.Text("Temperature:")),
						html.Span(html.ID("mem-temp"), g.Textf("%.0f°C", temps.MemoryTemp)),
					),
				),
			),
		),

		// Network Stats
		g.If(len(network) > 0,
			html.Div(
				html.Class("card"),
				html.H3(html.Class("text-base font-semibold mb-4"), g.Text("Network")),
				html.Div(
					html.Class("space-y-1 text-sm"),
					func() g.Node {
						idx := -1
						return g.Group(g.Map(network, func(iface system.NetworkStats) g.Node {
							idx++
							return g.El("details",
								html.Class("group border-b border-gray-100 dark:border-gray-800 last:border-0"),
								g.El("summary",
									html.Class("flex items-center justify-between py-2 cursor-pointer list-none"),
									html.Div(
										html.Class("flex items-center gap-2 min-w-0 flex-1"),
										netIfaceIcon(iface.Type),
										html.Span(html.Class("font-medium"), g.Text(iface.Interface)),
										html.Span(
											html.Class(ClassNames("badge", map[string]bool{
												"badge-success": iface.Status == "up",
												"badge-info":    iface.Status != "up",
											})),
											g.Text(iface.Status),
										),
									),
									html.Div(
										html.Class("flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400"),
										html.Div(
											html.Class("flex items-center gap-1"),
											html.Span(html.Class("text-green-500"), g.Text("↑")),
											html.Span(html.ID(fmt.Sprintf("net%d-txrate", idx)), g.Textf("%s/s", system.FormatBytes(iface.TxRate))),
										),
										html.Div(
											html.Class("flex items-center gap-1"),
											html.Span(html.Class("text-red-500"), g.Text("↓")),
											html.Span(html.ID(fmt.Sprintf("net%d-rxrate", idx)), g.Textf("%s/s", system.FormatBytes(iface.RxRate))),
										),
										g.Raw(`<svg class="w-3 h-3 transition-transform group-open:rotate-180 text-gray-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
									</svg>`),
									),
								),
								html.Div(
									html.Class("pb-3 pt-1 pl-6 space-y-2"),
									g.If(len(iface.RxHistory) > 0 || len(iface.TxHistory) > 0,
										html.Div(
											html.Class("flex gap-3"),
											html.Div(
												html.Class("flex-1"),
												html.Div(
													html.Class("flex justify-between items-baseline mb-0.5"),
													html.Span(html.Class("text-xs text-gray-400"), g.Text("RX")),
													html.Span(
														html.ID(fmt.Sprintf("net%d-rx-maxlabel", idx)),
														html.Class("text-[10px] text-gray-400 font-mono"),
														g.Text(netSparkMaxLabel(iface.RxHistory)),
													),
												),
												html.Div(
													html.ID(fmt.Sprintf("net%d-rx-spark", idx)),
													html.Class("h-12 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
													DataAttr("history", netSparkHistoryJSON(iface.RxHistory)),
													DataAttr("color", "#ef4444"),
													g.El("svg",
														html.Class("w-full h-full"),
														g.Attr("viewBox", "0 0 89 40"),
														g.Attr("preserveAspectRatio", "none"),
														g.El("path",
															g.Attr("d", netSparkFillPath(iface.RxHistory, 90, 40)),
															g.Attr("fill", "#ef4444"),
															g.Attr("fill-opacity", "0.15"),
															g.Attr("stroke", "none"),
														),
														g.El("path",
															g.Attr("d", netSparkLinePath(iface.RxHistory, 90, 40)),
															g.Attr("fill", "none"),
															g.Attr("stroke", "#ef4444"),
															g.Attr("stroke-width", "1.5"),
															g.Attr("stroke-linecap", "round"),
															g.Attr("stroke-linejoin", "round"),
															g.Attr("vector-effect", "non-scaling-stroke"),
														),
													),
												),
											),
											html.Div(
												html.Class("flex-1"),
												html.Div(
													html.Class("flex justify-between items-baseline mb-0.5"),
													html.Span(html.Class("text-xs text-gray-400"), g.Text("TX")),
													html.Span(
														html.ID(fmt.Sprintf("net%d-tx-maxlabel", idx)),
														html.Class("text-[10px] text-gray-400 font-mono"),
														g.Text(netSparkMaxLabel(iface.TxHistory)),
													),
												),
												html.Div(
													html.ID(fmt.Sprintf("net%d-tx-spark", idx)),
													html.Class("h-12 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
													DataAttr("history", netSparkHistoryJSON(iface.TxHistory)),
													DataAttr("color", "#22c55e"),
													g.El("svg",
														html.Class("w-full h-full"),
														g.Attr("viewBox", "0 0 89 40"),
														g.Attr("preserveAspectRatio", "none"),
														g.El("path",
															g.Attr("d", netSparkFillPath(iface.TxHistory, 90, 40)),
															g.Attr("fill", "#22c55e"),
															g.Attr("fill-opacity", "0.15"),
															g.Attr("stroke", "none"),
														),
														g.El("path",
															g.Attr("d", netSparkLinePath(iface.TxHistory, 90, 40)),
															g.Attr("fill", "none"),
															g.Attr("stroke", "#22c55e"),
															g.Attr("stroke-width", "1.5"),
															g.Attr("stroke-linecap", "round"),
															g.Attr("stroke-linejoin", "round"),
															g.Attr("vector-effect", "non-scaling-stroke"),
														),
													),
												),
											),
										),
									),
									html.Div(
										html.Class("text-xs text-gray-500 dark:text-gray-400 space-y-0.5"),
										g.If(iface.VLANID > 0,
											html.Div(
												html.Class("flex gap-2"),
												html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("VLAN ID")),
												html.Span(html.Class("font-mono"), g.Textf("%d", iface.VLANID)),
											),
										),
										g.If(iface.VLANParent != "",
											html.Div(
												html.Class("flex gap-2"),
												html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("Parent")),
												html.Span(html.Class("font-mono"), g.Text(iface.VLANParent)),
											),
										),
										g.If(iface.HardwareAddr != "",
											html.Div(
												html.Class("flex gap-2"),
												html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("MAC")),
												html.Span(html.Class("font-mono"), g.Text(iface.HardwareAddr)),
											),
										),
										g.Group(g.Map(iface.IPAddresses, func(addr string) g.Node {
											isIPv6 := strings.Contains(addr, ":")
											return html.Div(
												html.Class("flex gap-2 items-center"),
												g.If(isIPv6, html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("IPv6"))),
												g.If(!isIPv6, html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("IPv4"))),
												html.Span(html.Class("font-mono"), g.Text(addr)),
												g.If(iface.IPSource == "dhcp",
													html.Span(html.Class("badge badge-info text-[9px] px-1 py-0"), g.Text("DHCP")),
												),
												g.If(iface.IPSource == "static",
													html.Span(html.Class("badge badge-success text-[9px] px-1 py-0"), g.Text("Static")),
												),
											)
										})),
										g.If(iface.Gateway != "",
											html.Div(
												html.Class("flex gap-2"),
												html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("Gateway")),
												html.Span(html.Class("font-mono"), g.Text(iface.Gateway)),
											),
										),
										g.If(len(iface.DNSServers) > 0,
											html.Div(
												html.Class("flex gap-2"),
												html.Span(html.Class("text-gray-400 w-12 shrink-0"), g.Text("DNS")),
												html.Span(html.Class("font-mono"), g.Text(strings.Join(iface.DNSServers, ", "))),
											),
										),
									),
									html.Div(
										html.Class("flex gap-6 text-xs text-gray-600 dark:text-gray-400"),
										html.Div(
											html.Div(html.Class("text-gray-400 mb-0.5"), g.Text("Total sent")),
											html.Div(html.ID(fmt.Sprintf("net%d-sent", idx)), html.Class("font-medium"), g.Text(system.FormatBytes(iface.BytesSent))),
										),
										html.Div(
											html.Div(html.Class("text-gray-400 mb-0.5"), g.Text("Total recv")),
											html.Div(html.ID(fmt.Sprintf("net%d-recv", idx)), html.Class("font-medium"), g.Text(system.FormatBytes(iface.BytesRecv))),
										),
									),
								),
							)
						}))
					}(),
				),
			),
		),

		// GPU Stats
		func() g.Node {
			idx := -1
			return g.Group(g.Map(gpus, func(gpu system.GPUStats) g.Node {
				idx++
				return html.Div(
					html.Class("card"),
					html.H3(html.Class("text-base font-semibold mb-4"), g.Text("GPU")),
					html.Div(html.Class("text-sm font-medium mb-4"), g.Text(gpu.Name)),
					g.If(gpu.UtilizationGPU > 0,
						html.Div(
							html.Class("mb-4"),
							html.Div(
								html.Class("flex justify-between mb-1"),
								html.Span(html.Class("text-sm font-medium"), g.Text("GPU Usage")),
								html.Span(html.Class("text-sm font-medium"), html.ID(fmt.Sprintf("gpu%d-util-percent", idx)), g.Textf("%.0f%%", gpu.UtilizationGPU)),
							),
							html.Div(
								html.ID(fmt.Sprintf("gpu%d-util-spark", idx)),
								html.Class("h-16 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
								DataAttr("history", graphHistoryJSONf64(gpu.UtilHistory)),
								DataAttr("color", "#a855f7"),
								DataAttr("scale", "100"),
								g.El("svg",
									html.Class("w-full h-full"),
									g.Attr("viewBox", "0 0 89 40"),
									g.Attr("preserveAspectRatio", "none"),
									g.El("path",
										g.Attr("d", graphFillPath(gpu.UtilHistory, 90, 40, 100)),
										g.Attr("fill", "#a855f7"),
										g.Attr("fill-opacity", "0.15"),
										g.Attr("stroke", "none"),
									),
									g.El("path",
										g.Attr("d", graphLinePath(gpu.UtilHistory, 90, 40, 100)),
										g.Attr("fill", "none"),
										g.Attr("stroke", "#a855f7"),
										g.Attr("stroke-width", "1.5"),
										g.Attr("stroke-linecap", "round"),
										g.Attr("stroke-linejoin", "round"),
										g.Attr("vector-effect", "non-scaling-stroke"),
									),
								),
							),
						),
					),
					g.If(gpu.MemoryTotal > 0,
						html.Div(
							html.Class("mb-3"),
							html.Div(
								html.Class("flex justify-between mb-1"),
								html.Span(html.Class("text-sm font-medium"), g.Text("VRAM")),
								html.Span(html.Class("text-sm font-medium"), html.ID(fmt.Sprintf("gpu%d-mem-percent", idx)), g.Textf("%.1f%%", gpu.MemoryPercent)),
							),
							html.Div(
								html.ID(fmt.Sprintf("gpu%d-mem-spark", idx)),
								html.Class("h-16 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
								DataAttr("history", graphHistoryJSONf64(gpu.MemHistory)),
								DataAttr("color", "#f59e0b"),
								DataAttr("scale", "100"),
								g.El("svg",
									html.Class("w-full h-full"),
									g.Attr("viewBox", "0 0 89 40"),
									g.Attr("preserveAspectRatio", "none"),
									g.El("path",
										g.Attr("d", graphFillPath(gpu.MemHistory, 90, 40, 100)),
										g.Attr("fill", "#f59e0b"),
										g.Attr("fill-opacity", "0.15"),
										g.Attr("stroke", "none"),
									),
									g.El("path",
										g.Attr("d", graphLinePath(gpu.MemHistory, 90, 40, 100)),
										g.Attr("fill", "none"),
										g.Attr("stroke", "#f59e0b"),
										g.Attr("stroke-width", "1.5"),
										g.Attr("stroke-linecap", "round"),
										g.Attr("stroke-linejoin", "round"),
										g.Attr("vector-effect", "non-scaling-stroke"),
									),
								),
							),
							html.Div(
								html.Class("flex justify-between mt-1 text-xs text-gray-600 dark:text-gray-400"),
								html.Span(html.ID(fmt.Sprintf("gpu%d-mem-used", idx)), g.Textf("%s / %s", system.FormatBytes(gpu.MemoryUsed), system.FormatBytes(gpu.MemoryTotal))),
								html.Span(html.ID(fmt.Sprintf("gpu%d-mem-free", idx)), g.Textf("%s free", system.FormatBytes(gpu.MemoryTotal-gpu.MemoryUsed))),
							),
						),
					),
					g.If(gpu.Temperature > 0,
						html.Div(
							html.Class("mb-1"),
							html.Div(
								html.Class("flex justify-between text-sm text-gray-600 dark:text-gray-400 mb-1"),
								html.Span(g.Text("Temperature:")),
								html.Span(html.ID(fmt.Sprintf("gpu%d-temp", idx)), g.Textf("%.0f°C", gpu.Temperature)),
							),
							html.Div(
								html.ID(fmt.Sprintf("gpu%d-temp-spark", idx)),
								html.Class("h-12 bg-gray-50 dark:bg-gray-800 rounded overflow-hidden"),
								DataAttr("history", graphHistoryJSONf64(gpu.TempHistory)),
								DataAttr("scale", "120"),
								DataAttr("tempgraph", "1"),
								g.El("svg",
									html.Class("w-full h-full"),
									g.Attr("viewBox", "0 0 89 40"),
									g.Attr("preserveAspectRatio", "none"),
									g.El("path",
										g.Attr("d", graphFillPath(gpu.TempHistory, 90, 40, 120)),
										g.Attr("fill", "#3b82f6"),
										g.Attr("fill-opacity", "0.15"),
										g.Attr("stroke", "none"),
									),
									g.El("path",
										g.Attr("d", graphLinePath(gpu.TempHistory, 90, 40, 120)),
										g.Attr("fill", "none"),
										g.Attr("stroke", "#3b82f6"),
										g.Attr("stroke-width", "1.5"),
										g.Attr("stroke-linecap", "round"),
										g.Attr("stroke-linejoin", "round"),
										g.Attr("vector-effect", "non-scaling-stroke"),
									),
								),
							),
						),
					),
				)
			}))
		}(),

		// Disk Stats
		html.Div(
			html.Class("card lg:col-span-2"),
			html.H3(html.Class("text-base font-semibold mb-4"), g.Text("Storage")),
			html.Div(
				html.Class("grid grid-cols-1 md:grid-cols-2 gap-4"),
				func() g.Node {
					idx := -1
					return g.Group(g.Map(disks, func(disk system.DiskStats) g.Node {
						idx++
						return html.Div(
							html.Div(
								html.Class("flex justify-between mb-2"),
								html.Span(html.Class("text-sm font-medium"), g.Text(disk.MountPoint)),
								html.Span(html.Class("text-sm font-medium"), html.ID(fmt.Sprintf("disk%d-percent", idx)), g.Textf("%.1f%%", disk.UsedPercent)),
							),
							html.Div(
								html.Class("progress-bar"),
								html.Div(
									html.Class("progress-fill-gradient"),
									html.ID(fmt.Sprintf("disk%d-bar", idx)),
									DataAttr("percent", fmt.Sprintf("%.0f", disk.UsedPercent)),
									html.StyleAttr(fmt.Sprintf("width: %.1f%%", disk.UsedPercent)),
								),
							),
							html.Div(
								html.Class("flex justify-between mt-1 text-xs text-gray-600 dark:text-gray-400"),
								html.Span(html.ID(fmt.Sprintf("disk%d-used", idx)), g.Textf("%s / %s", system.FormatBytes(disk.Used), system.FormatBytes(disk.Total))),
								html.Span(html.ID(fmt.Sprintf("disk%d-free", idx)), g.Textf("%s free", system.FormatBytes(disk.Available))),
							),
						)
					}))
				}(),
			),
		),

		// Top Processes
		html.Div(
			html.Class("card lg:col-span-2"),
			html.ID("proc-card"),
			html.H3(html.Class("text-base font-semibold mb-3"), g.Text("Top Processes")),
			html.Div(
				html.Class("overflow-x-auto"),
				html.Table(
					html.Class("w-full text-sm table-fixed"),
					g.El("colgroup",
						g.El("col", html.StyleAttr("width:10%")),
						g.El("col", html.StyleAttr("width:40%")),
						g.El("col", html.StyleAttr("width:15%")),
						g.El("col", html.StyleAttr("width:20%")),
						g.El("col", html.StyleAttr("width:15%")),
					),
					html.THead(
						html.Tr(
							html.Class("text-left text-xs text-gray-500 dark:text-gray-400 border-b border-gray-200 dark:border-gray-700"),
							html.Th(
								html.Class("py-2 px-2 cursor-pointer select-none whitespace-nowrap"),
								g.Attr("onclick", "procSort('pid')"),
								g.Text("PID "),
								html.Span(html.ID("proc-arrow-pid"), html.Class("inline-block w-3 text-gray-400")),
							),
							html.Th(
								html.Class("py-2 px-2 cursor-pointer select-none whitespace-nowrap"),
								g.Attr("onclick", "procSort('name')"),
								g.Text("Name "),
								html.Span(html.ID("proc-arrow-name"), html.Class("inline-block w-3 text-gray-400")),
							),
							html.Th(
								html.Class("py-2 px-2 text-right cursor-pointer select-none whitespace-nowrap"),
								g.Attr("onclick", "procSort('cpu')"),
								g.Text("CPU% "),
								html.Span(html.ID("proc-arrow-cpu"), html.Class("inline-block w-3 text-gray-400")),
							),
							html.Th(
								html.Class("py-2 px-2 text-right cursor-pointer select-none whitespace-nowrap"),
								g.Attr("onclick", "procSort('mem')"),
								g.Text("Memory "),
								html.Span(html.ID("proc-arrow-mem"), html.Class("inline-block w-3 text-gray-400")),
							),
							html.Th(
								html.Class("py-2 px-2 text-right cursor-pointer select-none whitespace-nowrap"),
								g.Attr("onclick", "procSort('mempct')"),
								g.Text("Mem% "),
								html.Span(html.ID("proc-arrow-mempct"), html.Class("inline-block w-3 text-gray-400")),
							),
						),
					),
					html.TBody(
						html.ID("proc-tbody"),
						g.Group(g.Map(procs, func(proc system.ProcessInfo) g.Node {
							return html.Tr(
								html.Class("border-b border-gray-100 dark:border-gray-800"),
								DataAttr("pid", fmt.Sprintf("%d", proc.PID)),
								DataAttr("cpu", fmt.Sprintf("%.4f", proc.CPUPercent)),
								DataAttr("mem", fmt.Sprintf("%d", proc.MemoryBytes)),
								DataAttr("mempct", fmt.Sprintf("%.4f", proc.MemPercent)),
								html.Td(html.Class("py-2 px-2"), g.Textf("%d", proc.PID)),
								html.Td(html.Class("py-2 px-2 truncate max-w-xs"), g.Text(proc.Name)),
								html.Td(html.Class("py-2 px-2 text-right font-mono"), g.Textf("%.1f", proc.CPUPercent)),
								html.Td(html.Class("py-2 px-2 text-right"), g.Text(system.FormatBytes(proc.MemoryBytes))),
								html.Td(html.Class("py-2 px-2 text-right font-mono"), g.Textf("%.1f", proc.MemPercent)),
							)
						})),
					),
				),
			),
		),
	})
}
