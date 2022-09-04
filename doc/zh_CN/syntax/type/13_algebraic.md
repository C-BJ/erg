# Algebraic type

Algebraic types are types that are generated by operating types by treating them like algebra.
Operations handled by them include Union, Intersection, Diff, Complement, and so on.
Normal classes can only perform Union, and other operations will result in a type error.

## Union

Union types can give multiple possibilities for types. As the name suggests, they are generated by the `or` operator.
A typical Union is the `Option` type. The `Option` type is a `T or NoneType` patch type, primarily representing values that may fail.


```python
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# Implicitly become `T != NoneType`
Option T = T or NoneType
```

## Intersection

Intersection types are got by combining types with the `and` operation.

```python
Num = Add and Sub and Mul and Eq
```

As mentioned above, normal classes cannot be combined with the `and` operation. This is because instances belong to only one class.

## Diff

Diff types are got by `not` operation.
It is better to use `and not` as a closer notation to English text, but it is recommended to use just `not` because it fits better alongside `and` and `or`.

```python
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## Complement

Complement types is got by the `not` operation, which is a unary operation. The `not T` type is a shorthand for `{=} not T`.
Intersection with type `not T` is equivalent to Diff, and Diff with type `not T` is equivalent to Intersection.
However, this way of writing is not recommended.

```python
# the simplest definition of the non-zero number type
NonZero = Not {0}
# deprecated styles
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## True Algebraic type

There are two algebraic types: apparent algebraic types that can be simplified and true algebraic types that cannot be further simplified.
The "apparent algebraic types" include `or` and `and` of Enum, Interval, and the Record types.
These are not true algebraic types because they are simplified, and using them as type specifiers will result in a Warning; to eliminate the Warning, you must either simplify them or define their types.

```python
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # TypeWarning: {1, 2} or {3, 4} can be simplified to {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# TypeWaring: {x = Int, ...} and {y = Int; ...} can be simplified to {x = Int; y = Int; ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

True algebraic types include the types `Or` and `And`. Classes such as `or` between classes are of type `Or`.

```python
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff, Complement types are not true algebraic types because they can always be simplified.