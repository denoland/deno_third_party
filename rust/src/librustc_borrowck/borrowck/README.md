% The Borrow Checker

> WARNING: This README is more or less obsolete, and will be removed
> soon! The new system is described in the [rustc guide].

[rustc guide]: https://rust-lang.github.io/rustc-guide/borrow_check.html

This pass has the job of enforcing memory safety. This is a subtle
topic. This docs aim to explain both the practice and the theory
behind the borrow checker. They start with a high-level overview of
how it works, and then proceed to dive into the theoretical
background. Finally, they go into detail on some of the more subtle
aspects.

# Table of contents

These docs are long. Search for the section you are interested in.

- Overview
- Formal model
- Borrowing and loans
- Moves and initialization
- Drop flags and structural fragments
- Future work

# Overview

The borrow checker checks one function at a time. It operates in two
passes. The first pass, called `gather_loans`, walks over the function
and identifies all of the places where borrows (e.g., `&` expressions
and `ref` bindings) and moves (copies or captures of a linear value)
occur. It also tracks initialization sites. For each borrow and move,
it checks various basic safety conditions at this time (for example,
that the lifetime of the borrow doesn't exceed the lifetime of the
value being borrowed, or that there is no move out of an `&T`
referent).

It then uses the dataflow module to propagate which of those borrows
may be in scope at each point in the procedure. A loan is considered
to come into scope at the expression that caused it and to go out of
scope when the lifetime of the resulting reference expires.

Once the in-scope loans are known for each point in the program, the
borrow checker walks the IR again in a second pass called
`check_loans`. This pass examines each statement and makes sure that
it is safe with respect to the in-scope loans.

# Formal model

Throughout the docs we'll consider a simple subset of Rust in which
you can only borrow from places, defined like so:

```text
P = x | P.f | *P
```

Here `x` represents some variable, `P.f` is a field reference,
and `*P` is a pointer dereference. There is no auto-deref or other
niceties. This means that if you have a type like:

```rust
struct S { f: i32 }
```

and a variable `a: Box<S>`, then the rust expression `a.f` would correspond
to an `P` of `(*a).f`.

Here is the formal grammar for the types we'll consider:

```text
TY = i32 | bool | S<'LT...> | Box<TY> | & 'LT MQ TY
MQ = mut | imm
```

Most of these types should be pretty self explanatory. Here `S` is a
struct name and we assume structs are declared like so:

```text
SD = struct S<'LT...> { (f: TY)... }
```

# Borrowing and loans

## An intuitive explanation

### Issuing loans

Now, imagine we had a program like this:

```rust
struct Foo { f: i32, g: i32 }
...
'a: {
    let mut x: Box<Foo> = ...;
    let y = &mut (*x).f;
    x = ...;
}
```

This is of course dangerous because mutating `x` will free the old
value and hence invalidate `y`. The borrow checker aims to prevent
this sort of thing.

#### Loans and restrictions

The way the borrow checker works is that it analyzes each borrow
expression (in our simple model, that's stuff like `&P`, though in
real life there are a few other cases to consider). For each borrow
expression, it computes a `Loan`, which is a data structure that
records (1) the value being borrowed, (2) the mutability and scope of
the borrow, and (3) a set of restrictions. In the code, `Loan` is a
struct defined in `middle::borrowck`. Formally, we define `LOAN` as
follows:

```text
LOAN = (P, LT, MQ, RESTRICTION*)
RESTRICTION = (P, ACTION*)
ACTION = MUTATE | CLAIM | FREEZE
```

Here the `LOAN` tuple defines the place `P` being borrowed; the
lifetime `LT` of that borrow; the mutability `MQ` of the borrow; and a
list of restrictions. The restrictions indicate actions which, if
taken, could invalidate the loan and lead to type safety violations.

Each `RESTRICTION` is a pair of a restrictive place `P` (which will
either be the path that was borrowed or some prefix of the path that
was borrowed) and a set of restricted actions.  There are three kinds
of actions that may be restricted for the path `P`:

- `MUTATE` means that `P` cannot be assigned to;
- `CLAIM` means that the `P` cannot be borrowed mutably;
- `FREEZE` means that the `P` cannot be borrowed immutably;

Finally, it is never possible to move from a place that appears in a
restriction. This implies that the "empty restriction" `(P, [])`,
which contains an empty set of actions, still has a purpose---it
prevents moves from `P`. I chose not to make `MOVE` a fourth kind of
action because that would imply that sometimes moves are permitted
from restricted values, which is not the case.

#### Example

To give you a better feeling for what kind of restrictions derived
from a loan, let's look at the loan `L` that would be issued as a
result of the borrow `&mut (*x).f` in the example above:

```text
L = ((*x).f, 'a, mut, RS) where
    RS = [((*x).f, [MUTATE, CLAIM, FREEZE]),
          (*x, [MUTATE, CLAIM, FREEZE]),
          (x, [MUTATE, CLAIM, FREEZE])]
```

The loan states that the expression `(*x).f` has been loaned as
mutable for the lifetime `'a`. Because the loan is mutable, that means
that the value `(*x).f` may be mutated via the newly created reference
(and *only* via that pointer). This is reflected in the
restrictions `RS` that accompany the loan.

The first restriction `((*x).f, [MUTATE, CLAIM, FREEZE])` states that
the lender may not mutate, freeze, nor alias `(*x).f`. Mutation is
illegal because `(*x).f` is only supposed to be mutated via the new
reference, not by mutating the original path `(*x).f`. Freezing is
illegal because the path now has an `&mut` alias; so even if we the
lender were to consider `(*x).f` to be immutable, it might be mutated
via this alias. They will be enforced for the lifetime `'a` of the
loan. After the loan expires, the restrictions no longer apply.

The second restriction on `*x` is interesting because it does not
apply to the path that was lent (`(*x).f`) but rather to a prefix of
the borrowed path. This is due to the rules of inherited mutability:
if the user were to assign to (or freeze) `*x`, they would indirectly
overwrite (or freeze) `(*x).f`, and thus invalidate the reference
that was created. In general it holds that when a path is
lent, restrictions are issued for all the owning prefixes of that
path. In this case, the path `*x` owns the path `(*x).f` and,
because `x` has ownership, the path `x` owns the path `*x`.
Therefore, borrowing `(*x).f` yields restrictions on both
`*x` and `x`.

### Checking for illegal assignments, moves, and reborrows

Once we have computed the loans introduced by each borrow, the borrow
checker uses a data flow propagation to compute the full set of loans
in scope at each expression and then uses that set to decide whether
that expression is legal.  Remember that the scope of loan is defined
by its lifetime LT.  We sometimes say that a loan which is in-scope at
a particular point is an "outstanding loan", and the set of
restrictions included in those loans as the "outstanding
restrictions".

The kinds of expressions which in-scope loans can render illegal are:
- *assignments* (`lv = v`): illegal if there is an in-scope restriction
  against mutating `lv`;
- *moves*: illegal if there is any in-scope restriction on `lv` at all;
- *mutable borrows* (`&mut lv`): illegal there is an in-scope restriction
  against claiming `lv`;
- *immutable borrows* (`&lv`): illegal there is an in-scope restriction
  against freezing `lv`.

## Formal rules

Now that we hopefully have some kind of intuitive feeling for how the
borrow checker works, let's look a bit more closely now at the precise
conditions that it uses.

I will present the rules in a modified form of standard inference
rules, which looks as follows:

```text
PREDICATE(X, Y, Z)                  // Rule-Name
  Condition 1
  Condition 2
  Condition 3
```

The initial line states the predicate that is to be satisfied.  The
indented lines indicate the conditions that must be met for the
predicate to be satisfied. The right-justified comment states the name
of this rule: there are comments in the borrowck source referencing
these names, so that you can cross reference to find the actual code
that corresponds to the formal rule.

### Invariants

I want to collect, at a high-level, the invariants the borrow checker
maintains. I will give them names and refer to them throughout the
text. Together these invariants are crucial for the overall soundness
of the system.

**Mutability requires uniqueness.** To mutate a path

**Unique mutability.** There is only one *usable* mutable path to any
given memory at any given time. This implies that when claiming memory
with an expression like `p = &mut x`, the compiler must guarantee that
the borrowed value `x` can no longer be mutated so long as `p` is
live. (This is done via restrictions, read on.)

**.**


### The `gather_loans` pass

We start with the `gather_loans` pass, which walks the AST looking for
borrows.  For each borrow, there are three bits of information: the
place `P` being borrowed and the mutability `MQ` and lifetime `LT`
of the resulting pointer. Given those, `gather_loans` applies four
validity tests:

1. `MUTABILITY(P, MQ)`: The mutability of the reference is
compatible with the mutability of `P` (i.e., not borrowing immutable
data as mutable).

2. `ALIASABLE(P, MQ)`: The aliasability of the reference is
compatible with the aliasability of `P`. The goal is to prevent
`&mut` borrows of aliasability data.

3. `LIFETIME(P, LT, MQ)`: The lifetime of the borrow does not exceed
the lifetime of the value being borrowed.

4. `RESTRICTIONS(P, LT, ACTIONS) = RS`: This pass checks and computes the
restrictions to maintain memory safety. These are the restrictions
that will go into the final loan. We'll discuss in more detail below.

## Checking mutability

Checking mutability is fairly straightforward. We just want to prevent
immutable data from being borrowed as mutable. Note that it is ok to borrow
mutable data as immutable, since that is simply a freeze. The judgement
`MUTABILITY(P, MQ)` means the mutability of `P` is compatible with a borrow
of mutability `MQ`. The Rust code corresponding to this predicate is the
function `check_mutability` in `middle::borrowck::gather_loans`.

### Checking mutability of variables

*Code pointer:* Function `check_mutability()` in `gather_loans/mod.rs`,
but also the code in `mem_categorization`.

Let's begin with the rules for variables, which state that if a
variable is declared as mutable, it may be borrowed any which way, but
otherwise the variable must be borrowed as immutable:

```text
MUTABILITY(X, MQ)                   // M-Var-Mut
  DECL(X) = mut

MUTABILITY(X, imm)                  // M-Var-Imm
  DECL(X) = imm
```

### Checking mutability of owned content

Fields and boxes inherit their mutability from
their base expressions, so both of their rules basically
delegate the check to the base expression `P`:

```text
MUTABILITY(P.f, MQ)                // M-Field
  MUTABILITY(P, MQ)

MUTABILITY(*P, MQ)                 // M-Deref-Unique
  TYPE(P) = Box<Ty>
  MUTABILITY(P, MQ)
```

### Checking mutability of immutable pointer types

Immutable pointer types like `&T` can only
be borrowed if MQ is immutable:

```text
MUTABILITY(*P, imm)               // M-Deref-Borrowed-Imm
  TYPE(P) = &Ty
```

### Checking mutability of mutable pointer types

`&mut T` can be frozen, so it is acceptable to borrow it as either imm or mut:

```text
MUTABILITY(*P, MQ)                 // M-Deref-Borrowed-Mut
  TYPE(P) = &mut Ty
```

## Checking aliasability

The goal of the aliasability check is to ensure that we never permit `&mut`
borrows of aliasable data. The judgement `ALIASABLE(P, MQ)` means the
aliasability of `P` is compatible with a borrow of mutability `MQ`. The Rust
code corresponding to this predicate is the function `check_aliasability()` in
`middle::borrowck::gather_loans`.

### Checking aliasability of variables

Local variables are never aliasable as they are accessible only within
the stack frame.

```text
    ALIASABLE(X, MQ)                   // M-Var-Mut
```

### Checking aliasable of owned content

Owned content is aliasable if it is found in an aliasable location:

```text
ALIASABLE(P.f, MQ)                // M-Field
  ALIASABLE(P, MQ)

ALIASABLE(*P, MQ)                 // M-Deref-Unique
  ALIASABLE(P, MQ)
```

### Checking aliasability of immutable pointer types

Immutable pointer types like `&T` are aliasable, and hence can only be
borrowed immutably:

```text
ALIASABLE(*P, imm)                // M-Deref-Borrowed-Imm
  TYPE(P) = &Ty
```

### Checking aliasability of mutable pointer types

`&mut T` can be frozen, so it is acceptable to borrow it as either imm or mut:

```text
ALIASABLE(*P, MQ)                 // M-Deref-Borrowed-Mut
  TYPE(P) = &mut Ty
```

## Checking lifetime

These rules aim to ensure that no data is borrowed for a scope that exceeds
its lifetime. These two computations wind up being intimately related.
Formally, we define a predicate `LIFETIME(P, LT, MQ)`, which states that
"the place `P` can be safely borrowed for the lifetime `LT` with mutability
`MQ`". The Rust code corresponding to this predicate is the module
`middle::borrowck::gather_loans::lifetime`.

### Checking lifetime of variables

The rule for variables states that a variable can only be borrowed a
lifetime `LT` that is a subregion of the variable's scope:

```text
LIFETIME(X, LT, MQ)                 // L-Local
  LT <= block where X is declared
```

### Checking lifetime for owned content

The lifetime of a field or box is the same as the lifetime
of its owner:

```text
LIFETIME(P.f, LT, MQ)              // L-Field
  LIFETIME(P, LT, MQ)

LIFETIME(*P, LT, MQ)               // L-Deref-Send
  TYPE(P) = Box<Ty>
  LIFETIME(P, LT, MQ)
```

### Checking lifetime for derefs of references

References have a lifetime `LT'` associated with them.  The
data they point at has been guaranteed to be valid for at least this
lifetime. Therefore, the borrow is valid so long as the lifetime `LT`
of the borrow is shorter than the lifetime `LT'` of the pointer
itself:

```text
LIFETIME(*P, LT, MQ)               // L-Deref-Borrowed
  TYPE(P) = &LT' Ty OR &LT' mut Ty
  LT <= LT'
```

## Computing the restrictions

The final rules govern the computation of *restrictions*, meaning that
we compute the set of actions that will be illegal for the life of the
loan. The predicate is written `RESTRICTIONS(P, LT, ACTIONS) =
RESTRICTION*`, which can be read "in order to prevent `ACTIONS` from
occurring on `P`, the restrictions `RESTRICTION*` must be respected
for the lifetime of the loan".

Note that there is an initial set of restrictions: these restrictions
are computed based on the kind of borrow:

```text
&mut P =>   RESTRICTIONS(P, LT, MUTATE|CLAIM|FREEZE)
&P =>       RESTRICTIONS(P, LT, MUTATE|CLAIM)
```

The reasoning here is that a mutable borrow must be the only writer,
therefore it prevents other writes (`MUTATE`), mutable borrows
(`CLAIM`), and immutable borrows (`FREEZE`). An immutable borrow
permits other immutable borrows but forbids writes and mutable borrows.

### Restrictions for loans of a local variable

The simplest case is a borrow of a local variable `X`:

```text
RESTRICTIONS(X, LT, ACTIONS) = (X, ACTIONS)            // R-Variable
```

In such cases we just record the actions that are not permitted.

### Restrictions for loans of fields

Restricting a field is the same as restricting the owner of that
field:

```text
RESTRICTIONS(P.f, LT, ACTIONS) = RS, (P.f, ACTIONS)  // R-Field
  RESTRICTIONS(P, LT, ACTIONS) = RS
```

The reasoning here is as follows. If the field must not be mutated,
then you must not mutate the owner of the field either, since that
would indirectly modify the field. Similarly, if the field cannot be
frozen or aliased, we cannot allow the owner to be frozen or aliased,
since doing so indirectly freezes/aliases the field. This is the
origin of inherited mutability.

### Restrictions for loans of owned referents

Because the mutability of owned referents is inherited, restricting an
owned referent is similar to restricting a field, in that it implies
restrictions on the pointer. However, boxes have an important
twist: if the owner `P` is mutated, that causes the owned referent
`*P` to be freed! So whenever an owned referent `*P` is borrowed, we
must prevent the box `P` from being mutated, which means
that we always add `MUTATE` and `CLAIM` to the restriction set imposed
on `P`:

```text
RESTRICTIONS(*P, LT, ACTIONS) = RS, (*P, ACTIONS)    // R-Deref-Send-Pointer
  TYPE(P) = Box<Ty>
  RESTRICTIONS(P, LT, ACTIONS|MUTATE|CLAIM) = RS
```

### Restrictions for loans of immutable borrowed referents

Immutable borrowed referents are freely aliasable, meaning that
the compiler does not prevent you from copying the pointer.  This
implies that issuing restrictions is useless. We might prevent the
user from acting on `*P` itself, but there could be another path
`*P1` that refers to the exact same memory, and we would not be
restricting that path. Therefore, the rule for `&Ty` pointers
always returns an empty set of restrictions, and it only permits
restricting `MUTATE` and `CLAIM` actions:

```text
RESTRICTIONS(*P, LT, ACTIONS) = []                    // R-Deref-Imm-Borrowed
  TYPE(P) = &LT' Ty
  LT <= LT'                                            // (1)
  ACTIONS subset of [MUTATE, CLAIM]
```

The reason that we can restrict `MUTATE` and `CLAIM` actions even
without a restrictions list is that it is never legal to mutate nor to
borrow mutably the contents of a `&Ty` pointer. In other words,
those restrictions are already inherent in the type.

Clause (1) in the rule for `&Ty` deserves mention. Here I
specify that the lifetime of the loan must be less than the lifetime
of the `&Ty` pointer. In simple cases, this clause is redundant, since
the `LIFETIME()` function will already enforce the required rule:

```rust
fn foo(point: &'a Point) -> &'static i32 {
    &point.x // Error
}
```

The above example fails to compile both because of clause (1) above
but also by the basic `LIFETIME()` check. However, in more advanced
examples involving multiple nested pointers, clause (1) is needed:

```rust
fn foo(point: &'a &'b mut Point) -> &'b i32 {
    &point.x // Error
}
```

The `LIFETIME` rule here would accept `'b` because, in fact, the
*memory is* guaranteed to remain valid (i.e., not be freed) for the
lifetime `'b`, since the `&mut` pointer is valid for `'b`. However, we
are returning an immutable reference, so we need the memory to be both
valid and immutable. Even though `point.x` is referenced by an `&mut`
pointer, it can still be considered immutable so long as that `&mut`
pointer is found in an aliased location. That means the memory is
guaranteed to be *immutable* for the lifetime of the `&` pointer,
which is only `'a`, not `'b`. Hence this example yields an error.

As a final twist, consider the case of two nested *immutable*
pointers, rather than a mutable pointer within an immutable one:

```rust
fn foo(point: &'a &'b Point) -> &'b i32 {
    &point.x // OK
}
```

This function is legal. The reason for this is that the inner pointer
(`*point : &'b Point`) is enough to guarantee the memory is immutable
and valid for the lifetime `'b`.  This is reflected in
`RESTRICTIONS()` by the fact that we do not recurse (i.e., we impose
no restrictions on `P`, which in this particular case is the pointer
`point : &'a &'b Point`).

#### Why both `LIFETIME()` and `RESTRICTIONS()`?

Given the previous text, it might seem that `LIFETIME` and
`RESTRICTIONS` should be folded together into one check, but there is
a reason that they are separated. They answer separate concerns.
The rules pertaining to `LIFETIME` exist to ensure that we don't
create a borrowed pointer that outlives the memory it points at. So
`LIFETIME` prevents a function like this:

```rust
fn get_1<'a>() -> &'a i32 {
    let x = 1;
    &x
}
```

Here we would be returning a pointer into the stack. Clearly bad.

However, the `RESTRICTIONS` rules are more concerned with how memory
is used. The example above doesn't generate an error according to
`RESTRICTIONS` because, for local variables, we don't require that the
loan lifetime be a subset of the local variable lifetime. The idea
here is that we *can* guarantee that `x` is not (e.g.) mutated for the
lifetime `'a`, even though `'a` exceeds the function body and thus
involves unknown code in the caller -- after all, `x` ceases to exist
after we return and hence the remaining code in `'a` cannot possibly
mutate it. This distinction is important for type checking functions
like this one:

```rust
fn inc_and_get<'a>(p: &'a mut Point) -> &'a i32 {
    p.x += 1;
    &p.x
}
```

In this case, we take in a `&mut` and return a frozen borrowed pointer
with the same lifetime. So long as the lifetime of the returned value
doesn't exceed the lifetime of the `&mut` we receive as input, this is
fine, though it may seem surprising at first (it surprised me when I
first worked it through). After all, we're guaranteeing that `*p`
won't be mutated for the lifetime `'a`, even though we can't "see" the
entirety of the code during that lifetime, since some of it occurs in
our caller. But we *do* know that nobody can mutate `*p` except
through `p`. So if we don't mutate `*p` and we don't return `p`, then
we know that the right to mutate `*p` has been lost to our caller --
in terms of capability, the caller passed in the ability to mutate
`*p`, and we never gave it back. (Note that we can't return `p` while
`*p` is borrowed since that would be a move of `p`, as `&mut` pointers
are affine.)

### Restrictions for loans of mutable borrowed referents

Mutable borrowed pointers are guaranteed to be the only way to mutate
their referent. This permits us to take greater license with them; for
example, the referent can be frozen simply be ensuring that we do not
use the original pointer to perform mutate. Similarly, we can allow
the referent to be claimed, so long as the original pointer is unused
while the new claimant is live.

The rule for mutable borrowed pointers is as follows:

```text
RESTRICTIONS(*P, LT, ACTIONS) = RS, (*P, ACTIONS)    // R-Deref-Mut-Borrowed
  TYPE(P) = &LT' mut Ty
  LT <= LT'                                            // (1)
  RESTRICTIONS(P, LT, ACTIONS) = RS                   // (2)
```

Let's examine the two numbered clauses:

Clause (1) specifies that the lifetime of the loan (`LT`) cannot
exceed the lifetime of the `&mut` pointer (`LT'`). The reason for this
is that the `&mut` pointer is guaranteed to be the only legal way to
mutate its referent -- but only for the lifetime `LT'`.  After that
lifetime, the loan on the referent expires and hence the data may be
modified by its owner again. This implies that we are only able to
guarantee that the referent will not be modified or aliased for a
maximum of `LT'`.

Here is a concrete example of a bug this rule prevents:

```rust
// Test region-reborrow-from-shorter-mut-ref.rs:
fn copy_borrowed_ptr<'a,'b,T>(x: &'a mut &'b mut T) -> &'b mut T {
    &mut **p // ERROR due to clause (1)
}
fn main() {
    let mut x = 1;
    let mut y = &mut x; // <-'b-----------------------------+
    //      +-'a--------------------+                       |
    //      v                       v                       |
    let z = copy_borrowed_ptr(&mut y); // y is lent         |
    *y += 1; // Here y==z, so both should not be usable...  |
    *z += 1; // ...and yet they would be, but for clause 1. |
} // <------------------------------------------------------+
```

Clause (2) propagates the restrictions on the referent to the pointer
itself. This is the same as with an box, though the
reasoning is mildly different. The basic goal in all cases is to
prevent the user from establishing another route to the same data. To
see what I mean, let's examine various cases of what can go wrong and
show how it is prevented.

**Example danger 1: Moving the base pointer.** One of the simplest
ways to violate the rules is to move the base pointer to a new name
and access it via that new name, thus bypassing the restrictions on
the old name. Here is an example:

```rust
// src/test/compile-fail/borrowck-move-mut-base-ptr.rs
fn foo(t0: &mut i32) {
    let p: &i32 = &*t0; // Freezes `*t0`
    let t1 = t0;        //~ ERROR cannot move out of `t0`
    *t1 = 22;           // OK, not a write through `*t0`
}
```

Remember that `&mut` pointers are linear, and hence `let t1 = t0` is a
move of `t0` -- or would be, if it were legal. Instead, we get an
error, because clause (2) imposes restrictions on `P` (`t0`, here),
and any restrictions on a path make it impossible to move from that
path.

**Example danger 2: Claiming the base pointer.** Another possible
danger is to mutably borrow the base path. This can lead to two bad
scenarios. The most obvious is that the mutable borrow itself becomes
another path to access the same data, as shown here:

```rust
// src/test/compile-fail/borrowck-mut-borrow-of-mut-base-ptr.rs
fn foo<'a>(mut t0: &'a mut i32,
           mut t1: &'a mut i32) {
    let p: &i32 = &*t0;     // Freezes `*t0`
    let mut t2 = &mut t0;   //~ ERROR cannot borrow `t0`
    **t2 += 1;              // Mutates `*t0`
}
```

In this example, `**t2` is the same memory as `*t0`. Because `t2` is
an `&mut` pointer, `**t2` is a unique path and hence it would be
possible to mutate `**t2` even though that memory was supposed to be
frozen by the creation of `p`. However, an error is reported -- the
reason is that the freeze `&*t0` will restrict claims and mutation
against `*t0` which, by clause 2, in turn prevents claims and mutation
of `t0`. Hence the claim `&mut t0` is illegal.

Another danger with an `&mut` pointer is that we could swap the `t0`
value away to create a new path:

```rust
// src/test/compile-fail/borrowck-swap-mut-base-ptr.rs
fn foo<'a>(mut t0: &'a mut i32,
           mut t1: &'a mut i32) {
    let p: &i32 = &*t0;     // Freezes `*t0`
    swap(&mut t0, &mut t1); //~ ERROR cannot borrow `t0`
    *t1 = 22;
}
```

This is illegal for the same reason as above. Note that if we added
back a swap operator -- as we used to have -- we would want to be very
careful to ensure this example is still illegal.

**Example danger 3: Freeze the base pointer.** In the case where the
referent is claimed, even freezing the base pointer can be dangerous,
as shown in the following example:

```rust
// src/test/compile-fail/borrowck-borrow-of-mut-base-ptr.rs
fn foo<'a>(mut t0: &'a mut i32,
           mut t1: &'a mut i32) {
    let p: &mut i32 = &mut *t0; // Claims `*t0`
    let mut t2 = &t0;           //~ ERROR cannot borrow `t0`
    let q: &i32 = &*t2;         // Freezes `*t0` but not through `*p`
    *p += 1;                    // violates type of `*q`
}
```

Here the problem is that `*t0` is claimed by `p`, and hence `p` wants
to be the controlling pointer through which mutation or freezes occur.
But `t2` would -- if it were legal -- have the type `& &mut i32`, and
hence would be a mutable pointer in an aliasable location, which is
considered frozen (since no one can write to `**t2` as it is not a
unique path). Therefore, we could reasonably create a frozen `&i32`
pointer pointing at `*t0` that coexists with the mutable pointer `p`,
which is clearly unsound.

However, it is not always unsafe to freeze the base pointer. In
particular, if the referent is frozen, there is no harm in it:

```rust
// src/test/run-pass/borrowck-borrow-of-mut-base-ptr-safe.rs
fn foo<'a>(mut t0: &'a mut i32,
           mut t1: &'a mut i32) {
    let p: &i32 = &*t0; // Freezes `*t0`
    let mut t2 = &t0;
    let q: &i32 = &*t2; // Freezes `*t0`, but that's ok...
    let r: &i32 = &*t0; // ...after all, could do same thing directly.
}
```

In this case, creating the alias `t2` of `t0` is safe because the only
thing `t2` can be used for is to further freeze `*t0`, which is
already frozen. In particular, we cannot assign to `*t0` through the
new alias `t2`, as demonstrated in this test case:

```rust
// src/test/run-pass/borrowck-borrow-mut-base-ptr-in-aliasable-loc.rs
fn foo(t0: & &mut i32) {
    let t1 = t0;
    let p: &i32 = &**t0;
    **t1 = 22; //~ ERROR cannot assign
}
```

This distinction is reflected in the rules. When doing an `&mut`
borrow -- as in the first example -- the set `ACTIONS` will be
`CLAIM|MUTATE|FREEZE`, because claiming the referent implies that it
cannot be claimed, mutated, or frozen by anyone else. These
restrictions are propagated back to the base path and hence the base
path is considered unfreezable.

In contrast, when the referent is merely frozen -- as in the second
example -- the set `ACTIONS` will be `CLAIM|MUTATE`, because freezing
the referent implies that it cannot be claimed or mutated but permits
others to freeze. Hence when these restrictions are propagated back to
the base path, it will still be considered freezable.



**FIXME [RFC 1751](https://github.com/rust-lang/rfcs/issues/1751)
Restrictions against mutating the base pointer.**
When an `&mut` pointer is frozen or claimed, we currently pass along the
restriction against MUTATE to the base pointer. I do not believe this
restriction is needed. It dates from the days when we had a way to
mutate that preserved the value being mutated (i.e., swap). Nowadays
the only form of mutation is assignment, which destroys the pointer
being mutated -- therefore, a mutation cannot create a new path to the
same data. Rather, it removes an existing path. This implies that not
only can we permit mutation, we can have mutation kill restrictions in
the dataflow sense.

**WARNING:** We do not currently have `const` borrows in the
language. If they are added back in, we must ensure that they are
consistent with all of these examples. The crucial question will be
what sorts of actions are permitted with a `&const &mut` pointer. I
would suggest that an `&mut` referent found in an `&const` location be
prohibited from both freezes and claims. This would avoid the need to
prevent `const` borrows of the base pointer when the referent is
borrowed.

[ Previous revisions of this document discussed `&const` in more detail.
See the revision history. ]

# Moves and initialization

The borrow checker is also in charge of ensuring that:

- all memory which is accessed is initialized
- immutable local variables are assigned at most once.

These are two separate dataflow analyses built on the same
framework. Let's look at checking that memory is initialized first;
the checking of immutable local variable assignments works in a very
similar way.

To track the initialization of memory, we actually track all the
points in the program that *create uninitialized memory*, meaning
moves and the declaration of uninitialized variables. For each of
these points, we create a bit in the dataflow set. Assignments to a
variable `x` or path `a.b.c` kill the move/uninitialization bits for
those paths and any subpaths (e.g., `x`, `x.y`, `a.b.c`, `*a.b.c`).
Bits are unioned when two control-flow paths join. Thus, the
presence of a bit indicates that the move may have occurred without an
intervening assignment to the same memory. At each use of a variable,
we examine the bits in scope, and check that none of them are
moves/uninitializations of the variable that is being used.

Let's look at a simple example:

```rust
fn foo(a: Box<i32>) {
    let b: Box<i32>;   // Gen bit 0.

    if cond {          // Bits: 0
        use(&*a);
        b = a;         // Gen bit 1, kill bit 0.
        use(&*b);
    } else {
                       // Bits: 0
    }
                       // Bits: 0,1
    use(&*a);          // Error.
    use(&*b);          // Error.
}

fn use(a: &i32) { }
```

In this example, the variable `b` is created uninitialized. In one
branch of an `if`, we then move the variable `a` into `b`. Once we
exit the `if`, therefore, it is an error to use `a` or `b` since both
are only conditionally initialized. I have annotated the dataflow
state using comments. There are two dataflow bits, with bit 0
corresponding to the creation of `b` without an initializer, and bit 1
corresponding to the move of `a`. The assignment `b = a` both
generates bit 1, because it is a move of `a`, and kills bit 0, because
`b` is now initialized. On the else branch, though, `b` is never
initialized, and so bit 0 remains untouched. When the two flows of
control join, we union the bits from both sides, resulting in both
bits 0 and 1 being set. Thus any attempt to use `a` uncovers the bit 1
from the "then" branch, showing that `a` may be moved, and any attempt
to use `b` uncovers bit 0, from the "else" branch, showing that `b`
may not be initialized.

## Initialization of immutable variables

Initialization of immutable variables works in a very similar way,
except that:

1. we generate bits for each assignment to a variable;
2. the bits are never killed except when the variable goes out of scope.

Thus the presence of an assignment bit indicates that the assignment
may have occurred. Note that assignments are only killed when the
variable goes out of scope, as it is not relevant whether or not there
has been a move in the meantime. Using these bits, we can declare that
an assignment to an immutable variable is legal iff there is no other
assignment bit to that same variable in scope.

## Why is the design made this way?

It may seem surprising that we assign dataflow bits to *each move*
rather than *each path being moved*. This is somewhat less efficient,
since on each use, we must iterate through all moves and check whether
any of them correspond to the path in question. Similar concerns apply
to the analysis for double assignments to immutable variables. The
main reason to do it this way is that it allows us to print better
error messages, because when a use occurs, we can print out the
precise move that may be in scope, rather than simply having to say
"the variable may not be initialized".

## Data structures used in the move analysis

The move analysis maintains several data structures that enable it to
cross-reference moves and assignments to determine when they may be
moving/assigning the same memory. These are all collected into the
`MoveData` and `FlowedMoveData` structs. The former represents the set
of move paths, moves, and assignments, and the latter adds in the
results of a dataflow computation.

### Move paths

The `MovePath` tree tracks every path that is moved or assigned to.
These paths have the same form as the `LoanPath` data structure, which
in turn is the "real world version of the places `P` that we
introduced earlier. The difference between a `MovePath` and a `LoanPath`
is that move paths are:

1. Canonicalized, so that we have exactly one copy of each, and
   we can refer to move paths by index;
2. Cross-referenced with other paths into a tree, so that given a move
   path we can efficiently find all parent move paths and all
   extensions (e.g., given the `a.b` move path, we can easily find the
   move path `a` and also the move paths `a.b.c`)
3. Cross-referenced with moves and assignments, so that we can
   easily find all moves and assignments to a given path.

The mechanism that we use is to create a `MovePath` record for each
move path. These are arranged in an array and are referenced using
`MovePathIndex` values, which are newtype'd indices. The `MovePath`
structs are arranged into a tree, representing using the standard
Knuth representation where each node has a child 'pointer' and a "next
sibling" 'pointer'. In addition, each `MovePath` has a parent
'pointer'.  In this case, the 'pointers' are just `MovePathIndex`
values.

In this way, if we want to find all base paths of a given move path,
we can just iterate up the parent pointers (see `each_base_path()` in
the `move_data` module). If we want to find all extensions, we can
iterate through the subtree (see `each_extending_path()`).

### Moves and assignments

There are structs to represent moves (`Move`) and assignments
(`Assignment`), and these are also placed into arrays and referenced
by index. All moves of a particular path are arranged into a linked
lists, beginning with `MovePath.first_move` and continuing through
`Move.next_move`.

We distinguish between "var" assignments, which are assignments to a
variable like `x = foo`, and "path" assignments (`x.f = foo`).  This
is because we need to assign dataflows to the former, but not the
latter, so as to check for double initialization of immutable
variables.

### Gathering and checking moves

Like loans, we distinguish two phases. The first, gathering, is where
we uncover all the moves and assignments. As with loans, we do some
basic sanity checking in this phase, so we'll report errors if you
attempt to move out of a borrowed pointer etc. Then we do the dataflow
(see `FlowedMoveData::new`). Finally, in the `check_loans.rs` code, we
walk back over, identify all uses, assignments, and captures, and
check that they are legal given the set of dataflow bits we have
computed for that program point.

# Drop flags and structural fragments

In addition to the job of enforcing memory safety, the borrow checker
code is also responsible for identifying the *structural fragments* of
data in the function, to support out-of-band dynamic drop flags
allocated on the stack. (For background, see [RFC PR #320].)

[RFC PR #320]: https://github.com/rust-lang/rfcs/pull/320

Semantically, each piece of data that has a destructor may need a
boolean flag to indicate whether or not its destructor has been run
yet. However, in many cases there is no need to actually maintain such
a flag: It can be apparent from the code itself that a given path is
always initialized (or always deinitialized) when control reaches the
end of its owner's scope, and thus we can unconditionally emit (or
not) the destructor invocation for that path.

A simple example of this is the following:

```rust
struct D { p: i32 }
impl D { fn new(x: i32) -> D { ... }
impl Drop for D { ... }

fn foo(a: D, b: D, t: || -> bool) {
    let c: D;
    let d: D;
    if t() { c = b; }
}
```

At the end of the body of `foo`, the compiler knows that `a` is
initialized, introducing a drop obligation (deallocating the boxed
integer) for the end of `a`'s scope that is run unconditionally.
Likewise the compiler knows that `d` is not initialized, and thus it
leave out the drop code for `d`.

The compiler cannot statically know the drop-state of `b` nor `c` at
the end of their scope, since that depends on the value of
`t`. Therefore, we need to insert boolean flags to track whether we
need to drop `b` and `c`.

However, the matter is not as simple as just mapping local variables
to their corresponding drop flags when necessary. In particular, in
addition to being able to move data out of local variables, Rust
allows one to move values in and out of structured data.

Consider the following:

```rust
struct S { x: D, y: D, z: D }

fn foo(a: S, mut b: S, t: || -> bool) {
    let mut c: S;
    let d: S;
    let e: S = a.clone();
    if t() {
        c = b;
        b.x = e.y;
    }
    if t() { c.y = D::new(4); }
}
```

As before, the drop obligations of `a` and `d` can be statically
determined, and again the state of `b` and `c` depend on dynamic
state. But additionally, the dynamic drop obligations introduced by
`b` and `c` are not just per-local boolean flags. For example, if the
first call to `t` returns `false` and the second call `true`, then at
the end of their scope, `b` will be completely initialized, but only
`c.y` in `c` will be initialized.  If both calls to `t` return `true`,
then at the end of their scope, `c` will be completely initialized,
but only `b.x` will be initialized in `b`, and only `e.x` and `e.z`
will be initialized in `e`.

Note that we need to cover the `z` field in each case in some way,
since it may (or may not) need to be dropped, even though `z` is never
directly mentioned in the body of the `foo` function. We call a path
like `b.z` a *fragment sibling* of `b.x`, since the field `z` comes
from the same structure `S` that declared the field `x` in `b.x`.

In general we need to maintain boolean flags that match the
`S`-structure of both `b` and `c`.  In addition, we need to consult
such a flag when doing an assignment (such as `c.y = D::new(4);`
above), in order to know whether or not there is a previous value that
needs to be dropped before we do the assignment.

So for any given function, we need to determine what flags are needed
to track its drop obligations. Our strategy for determining the set of
flags is to represent the fragmentation of the structure explicitly:
by starting initially from the paths that are explicitly mentioned in
moves and assignments (such as `b.x` and `c.y` above), and then
traversing the structure of the path's type to identify leftover
*unmoved fragments*: assigning into `c.y` means that `c.x` and `c.z`
are leftover unmoved fragments. Each fragment represents a drop
obligation that may need to be tracked. Paths that are only moved or
assigned in their entirety (like `a` and `d`) are treated as a single
drop obligation.

The fragment construction process works by piggy-backing on the
existing `move_data` module. We already have callbacks that visit each
direct move and assignment; these form the basis for the sets of
moved_leaf_paths and assigned_leaf_paths. From these leaves, we can
walk up their parent chain to identify all of their parent paths.
We need to identify the parents because of cases like the following:

```rust
struct Pair<X,Y>{ x: X, y: Y }
fn foo(dd_d_d: Pair<Pair<Pair<D, D>, D>, D>) {
    other_function(dd_d_d.x.y);
}
```

In this code, the move of the path `dd_d.x.y` leaves behind not only
the fragment drop-obligation `dd_d.x.x` but also `dd_d.y` as well.

Once we have identified the directly-referenced leaves and their
parents, we compute the left-over fragments, in the function
`fragments::add_fragment_siblings`. As of this writing this works by
looking at each directly-moved or assigned path P, and blindly
gathering all sibling fields of P (as well as siblings for the parents
of P, etc). After accumulating all such siblings, we filter out the
entries added as siblings of P that turned out to be
directly-referenced paths (or parents of directly referenced paths)
themselves, thus leaving the never-referenced "left-overs" as the only
thing left from the gathering step.

## Array structural fragments

A special case of the structural fragments discussed above are
the elements of an array that has been passed by value, such as
the following:

```rust
fn foo(a: [D; 10], i: i32) -> D {
    a[i]
}
```

The above code moves a single element out of the input array `a`.
The remainder of the array still needs to be dropped; i.e., it
is a structural fragment. Note that after performing such a move,
it is not legal to read from the array `a`. There are a number of
ways to deal with this, but the important thing to note is that
the semantics needs to distinguish in some manner between a
fragment that is the *entire* array versus a fragment that represents
all-but-one element of the array.  A place where that distinction
would arise is the following:

```rust
fn foo(a: [D; 10], b: [D; 10], i: i32, t: bool) -> D {
    if t {
        a[i]
    } else {
        b[i]
    }

    // When control exits, we will need either to drop all of `a`
    // and all-but-one of `b`, or to drop all of `b` and all-but-one
    // of `a`.
}
```

There are a number of ways that the codegen backend could choose to
compile this (e.g. a `[bool; 10]` array for each such moved array;
or an `Option<usize>` for each moved array).  From the viewpoint of the
borrow-checker, the important thing is to record what kind of fragment
is implied by the relevant moves.

# Future work

While writing up these docs, I encountered some rules I believe to be
stricter than necessary:

- I think restricting the `&mut` P against moves and `ALIAS` is sufficient,
  `MUTATE` and `CLAIM` are overkill. `MUTATE` was necessary when swap was
  a built-in operator, but as it is not, it is implied by `CLAIM`,
  and `CLAIM` is implied by `ALIAS`. The only net effect of this is an
  extra error message in some cases, though.
- I have not described how closures interact. Current code is unsound.
  I am working on describing and implementing the fix.
- If we wish, we can easily extend the move checking to allow finer-grained
  tracking of what is initialized and what is not, enabling code like
  this:

      a = x.f.g; // x.f.g is now uninitialized
      // here, x and x.f are not usable, but x.f.h *is*
      x.f.g = b; // x.f.g is not initialized
      // now x, x.f, x.f.g, x.f.h are all usable

  What needs to change here, most likely, is that the `moves` module
  should record not only what paths are moved, but what expressions
  are actual *uses*. For example, the reference to `x` in `x.f.g = b`
  is not a true *use* in the sense that it requires `x` to be fully
  initialized. This is in fact why the above code produces an error
  today: the reference to `x` in `x.f.g = b` is considered illegal
  because `x` is not fully initialized.

There are also some possible refactorings:

- It might be nice to replace all loan paths with the MovePath mechanism,
  since they allow lightweight comparison using an integer.
