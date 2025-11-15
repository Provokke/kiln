function iter(cx, cy, max) {
  let x = 0, y = 0, i = 0;
  while (i < max) { const xx = x * x, yy = y * y; if (xx + yy > 4) return i; y = 2 * x * y + cy; x = xx - yy + cx; i = i + 1; }
  return max;
}
let total = 0;
for (let py = 0; py < 200; py++) for (let px = 0; px < 300; px++) total = total + iter(-2.2 + px * 0.01, -1.2 + py * 0.012, 200);
console.log(total);
