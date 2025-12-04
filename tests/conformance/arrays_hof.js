const xs = [1, 2, 3, 4, 5, 6];
console.log(xs.map(x => x * x), xs.filter(x => x % 2 === 0), xs.reduce((a, b) => a + b, 0));
console.log(xs.some(x => x > 5), xs.every(x => x > 0), xs.find(x => x > 3), xs.findIndex(x => x > 3));
let sum = 0; xs.forEach((x, i) => { sum += x * i; }); console.log(sum);
console.log(xs.map((x, i) => [i, x]).filter(p => p[0] % 3 === 0));
console.log(["b", "a", "c"].map(s => s.toUpperCase()).sort().join(""));
console.log([[1, 2], [3]].reduce((acc, v) => acc.concat(v), []));
