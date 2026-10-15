const o = { a: 1, "b-c": 2, 3: "three", nested: { x: [1, 2] } };
o.d = 4; o["e"] = 5;
console.log(o, o.a, o["b-c"], o[3], o.nested.x[1], o.missing);
console.log(Object.keys(o), "a" in o, "zz" in o);
const k = "dyn"; const p = { [k + "1"]: 1, k, ...{ s: 1, t: 2 } };
console.log(p, JSON.stringify(p));
for (const key in { x: 1, y: 2 }) console.log(key);
console.log(Object.values({ q: 1, r: [2] }), Object.entries({ m: 1 }));
