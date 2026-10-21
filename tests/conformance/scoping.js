let x = "global";
{ let x = "block"; console.log(x); }
console.log(x);
function f() { var a = 1; if (true) { var a = 2; } return a; }
console.log(f());
function g() { let a = 1; { let a = 2; } return a; }
console.log(g());
function h() { return typeof later; var later = 1; }
console.log(h());
var gv = 10; function readGv() { return gv; } gv = 11; console.log(readGv());
const fns = []; for (var i = 0; i < 3; i++) fns.push(() => i); console.log(fns.map(q => q()));
