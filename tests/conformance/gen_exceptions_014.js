function risky14(n) {
  if (n === 2) throw new Error("bad " + n);
  if (n > 5) throw new RangeError("range " + n);
  return n * 2;
}
const out = [];
for (let j = 0; j < 8; j++) {
  try { out.push(risky14(j)); }
  catch (e) { out.push(e.name + ":" + e.message); }
  finally { if (j % 3 === 0) out.push("f"); }
}
console.log(out.join(","));
function nested14() { try { try { risky14(2); } catch (e) { throw new TypeError("wrapped " + e.message); } } catch (e2) { return e2.name + "/" + e2.message; } }
console.log(nested14());
