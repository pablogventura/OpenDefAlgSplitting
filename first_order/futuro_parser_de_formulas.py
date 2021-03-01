import sys
from enum import Enum

class Type(Enum):  # This could also be done with individual classes
    leftparentheses = 0
    rightparentheses = 1
    operator = 2
    empty = 3
    operand = 4

OPERATORS = {  # get your data out of your code...
    "+": "add",
    "-": "subtract",
    "*": "multiply",
    "%": "modulus",
    "/": "divide",
}

def textOperator(string):
    if string not in OPERATORS:
        sys.exit("Unknown operator: " + string)
    return OPERATORS[string]

def typeof(string):
    if string == '(':
        return Type.leftparentheses
    elif string == ')':
        return Type.rightparentheses
    elif string in OPERATORS:
        return Type.operator
    elif string == ' ':
        return Type.empty
    else:
        return Type.operand

def process(tokens):

    stack = []

    while tokens:
        token = tokens.pop()

        category = typeof(token)

        print("token = ", token, " (" + str(category) + ")")

        if category == Type.operand:
            stack.append(token)
        elif category == Type.operator:
            stack.append((textOperator(token), stack.pop(), process(tokens)))
        elif category == Type.leftparentheses:
            stack.append(process(tokens))
        elif category == Type.rightparentheses:
            return stack.pop()
        elif category == Type.empty:
            continue

        print("stack = ", stack)

    return stack.pop()

INFIX = "1 + ((C + A ) * (B - F))"

# pop/append work from right, so reverse, and require a real list
postfix = process(list(INFIX[::-1]))

print(postfix)
