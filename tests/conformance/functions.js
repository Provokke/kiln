function add(a, b) { return a + b; }
console.log(add(1, 2), add(1), add(1, 2, 3), add("a", "b"));
const sq = function (x) { return x * x; };
const fact = function f(n) { return n <= 1 ? 1 : n * f(n - 1); };
console.log(sq(5), fact(10), sq.name, fact.name, add.name, (() => 1).name);
console.log(hoisted());
function hoisted() { return "hoisted!"; }
const arrow = (a, b) => ({ sum: a + b });
console.log(arrow(2, 3), ((x) => x * 2)(4), add, sq, class Q {});
function rec(n) { return n === 0 ? 0 : 1 + rec(n - 1); }
console.log(rec(1000));
function three(a, b, c) { return [a, b, c]; }
console.log(three(...[1, 2, 3]), Math.max(...[4, 9, 2]), [0, ...[1, 2], 3]);
