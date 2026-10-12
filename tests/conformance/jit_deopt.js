// int32 speculation: overflow, -0, fractional and negative-modulo operands all deopt
function mul(a, b) { return a * b; }
for (let i = 0; i < 50; i++) mul(i, i);
console.log(mul(65536, 65536), mul(0, -5), mul(-3, 0), mul(2.5, 2), mul(3, 4));
function addl(a, b) { return a + b; }
for (let i = 0; i < 50; i++) addl(i, 1);
console.log(addl(2147483647, 1), addl(-2147483648, -1), addl(0.5, 0.25), addl(1, 2));
function md(a, b) { return a % b; }
for (let i = 0; i < 50; i++) md(i, 7);
console.log(md(-7, 3), md(7, -3), md(5, 0), md(-4, 2), md(9, 4));
function ng(a) { return -a; }
for (let i = 1; i < 50; i++) ng(i);
console.log(ng(0), ng(-2147483648), ng(7));
function loopOverflow(n) { let x = 1; for (let i = 0; i < n; i++) { x = x * 2; } return x; }
for (let i = 0; i < 30; i++) loopOverflow(i);
console.log(loopOverflow(10), loopOverflow(31), loopOverflow(40), loopOverflow(64));
function noReturn(n) { n = n + 1; }
for (let i = 0; i < 30; i++) noReturn(i);
console.log(noReturn(3));
function callsNonNum(n) { return helperS(n) + 1; }
function helperS(n) { return n > 5 ? "s" : n; }
for (let i = 0; i < 30; i++) callsNonNum(1);
console.log(callsNonNum(2), callsNonNum(9));
function throwsInside(n) { return thrower(n) * 2; }
function thrower(n) { if (n > 10) throw new Error("boom " + n); return n; }
for (let i = 0; i < 30; i++) throwsInside(i % 5);
try { throwsInside(11); } catch (e) { console.log(e.message); }
