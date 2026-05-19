function risky16(n) {
  if (n === 5) throw new Error("bad " + n);
  if (n > 8) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 11; j++) {
  try { out.push(risky16(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested16() { try { try { risky16(5); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested16());
