function risky7(n) {
  if (n === 8) throw new Error("bad " + n);
  if (n > 11) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 14; j++) {
  try { out.push(risky7(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested7() { try { try { risky7(8); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested7());
