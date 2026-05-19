function risky11(n) {
  if (n === 4) throw new Error("bad " + n);
  if (n > 7) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 10; j++) {
  try { out.push(risky11(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested11() { try { try { risky11(4); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested11());
