function inf(n) { return inf(n + 1) + 1; }
try { inf(0); } catch (e) { console.log(e instanceof RangeError, e.message); }
function deep(n) { return n === 0 ? 0 : 1 + deep(n - 1); }
console.log(deep(5000));
