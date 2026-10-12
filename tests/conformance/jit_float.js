function mandel(cx, cy, maxIter) {
  let x = 0, y = 0, i = 0;
  while (i < maxIter) {
    const xx = x * x, yy = y * y;
    if (xx + yy > 4) return i;
    y = 2 * x * y + cy;
    x = xx - yy + cx;
    i = i + 1;
  }
  return maxIter;
}
let rows = [];
for (let py = 0; py < 12; py++) {
  let row = "";
  for (let px = 0; px < 30; px++) {
    const it = mandel(-2 + px * 0.09, -1.1 + py * 0.2, 50);
    row += it >= 50 ? "#" : it > 5 ? "+" : ".";
  }
  rows.push(row);
}
console.log(rows.join("\n"));
function cmpf(a, b) { if (a < b) return -1; if (a > b) return 1; if (a === b) return 0; return 99; }
for (let i = 0; i < 30; i++) cmpf(i * 0.5, 3.25);
console.log(cmpf(1.5, 2.5), cmpf(2.5, 1.5), cmpf(0.5, 0.5), cmpf(NaN, 1), cmpf(1, NaN), cmpf(-0, 0));
function truthy(x) { if (x) return 1; return 0; }
for (let i = 0; i < 30; i++) truthy(i * 0.1);
console.log(truthy(0), truthy(-0), truthy(NaN), truthy(0.1), truthy(-3));
function divs(a, b) { return a / b; }
for (let i = 1; i < 30; i++) divs(i, 3);
console.log(divs(1, 0), divs(-1, 0), divs(0, 0), divs(7, 2), divs(-0, 5));
