function risky9(n) {
  if (n === 1) throw new Error("bad " + n);
  if (n > 4) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 7; j++) {
  try { out.push(risky9(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested9() { try { try { risky9(1); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested9());
