function risky17(n) {
  if (n === 6) throw new Error("bad " + n);
  if (n > 9) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 12; j++) {
  try { out.push(risky17(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested17() { try { try { risky17(6); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested17());
