console.log(JSON.stringify({ a: 1, b: [true, false, null], c: "q\"uote\n", d: undefined, e: () => 1 }));
console.log(JSON.stringify([1, undefined, function () {}, NaN, Infinity]));
console.log(JSON.stringify("str"), JSON.stringify(42), JSON.stringify(null), JSON.stringify(undefined));
console.log(JSON.stringify({ x: { y: { z: [1, { w: 2 }] } } }, null, 2));
console.log(JSON.stringify([], null, 2), JSON.stringify({}, null, 2), JSON.stringify([[]], null, 1));
