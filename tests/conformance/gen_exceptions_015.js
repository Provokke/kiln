function risky15(n) {
  if (n === 7) throw new Error("bad " + n);
  if (n > 10) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 13; j++) {
  try { out.push(risky15(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested15() { try { try { risky15(7); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested15());
