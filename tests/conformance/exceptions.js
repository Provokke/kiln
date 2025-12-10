function f(x) { if (x > 2) throw new Error("too big: " + x); return x; }
let log = [];
for (let i = 0; i < 5; i++) {
  try { log.push(f(i)); } catch (e) { log.push(e.message); } finally { log.push("f" + i); }
}
console.log(log.join("|"));
try { try { throw 1; } finally { console.log("inner finally"); } } catch (e) { console.log("caught", e); }
function g() { try { return "try"; } catch (e) { return "catch"; } }
console.log(g());
try { throw { code: 42 }; } catch (e) { console.log(e.code); }
const err = new RangeError("r");
console.log(err.name, err.message, err instanceof Error, String(err));
