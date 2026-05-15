function risky6(n) {
  if (n === 3) throw new Error("bad " + n);
  if (n > 6) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 9; j++) {
  try { out.push(risky6(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested6() { try { try { risky6(3); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested6());
