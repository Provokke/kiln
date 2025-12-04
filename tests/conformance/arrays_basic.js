const a = [3, 1, 2];
a.push(5, 4);
console.log(a, a.length, a.pop(), a.shift(), a);
console.log(a.indexOf(2), a.includes(9), a.slice(1), a.concat([7, 8], 9));
a.unshift(0); a[4] = 6;
console.log(a, a.length);
console.log([1, 2, 3].reverse(), [5, 1, 10, 2].sort(), [5, 1, 10, 2].sort((x, y) => x - y));
console.log([1, [2, [3, [4]]]], [], [[]], Array.isArray(a), Array.isArray("a"));
const b = [1, 2, 3, 4, 5]; b.length = 2; console.log(b, b.splice(0, 1), b);
