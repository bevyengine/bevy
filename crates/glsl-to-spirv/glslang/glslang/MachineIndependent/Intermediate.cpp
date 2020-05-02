//
// Copyright (C) 2002-2005  3Dlabs Inc. Ltd.
// Copyright (C) 2012-2015 LunarG, Inc.
// Copyright (C) 2015-2016 Google, Inc.
//
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//    Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
//    Redistributions in binary form must reproduce the above
//    copyright notice, this list of conditions and the following
//    disclaimer in the documentation and/or other materials provided
//    with the distribution.
//
//    Neither the name of 3Dlabs Inc. Ltd. nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
// FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
// COPYRIGHT HOLDERS OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
// INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
// ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//

//
// Build the intermediate representation.
//

#include "localintermediate.h"
#include "RemoveTree.h"
#include "SymbolTable.h"
#include "propagateNoContraction.h"

#include <cfloat>
#include <utility>

namespace glslang {

////////////////////////////////////////////////////////////////////////////
//
// First set of functions are to help build the intermediate representation.
// These functions are not member functions of the nodes.
// They are called from parser productions.
//
/////////////////////////////////////////////////////////////////////////////

//
// Add a terminal node for an identifier in an expression.
//
// Returns the added node.
//

TIntermSymbol* TIntermediate::addSymbol(int id, const TString& name, const TType& type, const TConstUnionArray& constArray,
                                        TIntermTyped* constSubtree, const TSourceLoc& loc)
{
    TIntermSymbol* node = new TIntermSymbol(id, name, type);
    node->setLoc(loc);
    node->setConstArray(constArray);
    node->setConstSubtree(constSubtree);

    return node;
}

TIntermSymbol* TIntermediate::addSymbol(const TIntermSymbol& intermSymbol)
{
    return addSymbol(intermSymbol.getId(),
                     intermSymbol.getName(),
                     intermSymbol.getType(),
                     intermSymbol.getConstArray(),
                     intermSymbol.getConstSubtree(),
                     intermSymbol.getLoc());
}

TIntermSymbol* TIntermediate::addSymbol(const TVariable& variable)
{
    glslang::TSourceLoc loc; // just a null location
    loc.init();

    return addSymbol(variable, loc);
}

TIntermSymbol* TIntermediate::addSymbol(const TVariable& variable, const TSourceLoc& loc)
{
    return addSymbol(variable.getUniqueId(), variable.getName(), variable.getType(), variable.getConstArray(), variable.getConstSubtree(), loc);
}

TIntermSymbol* TIntermediate::addSymbol(const TType& type, const TSourceLoc& loc)
{
    TConstUnionArray unionArray;  // just a null constant

    return addSymbol(0, "", type, unionArray, nullptr, loc);
}

//
// Connect two nodes with a new parent that does a binary operation on the nodes.
//
// Returns the added node.
//
// Returns nullptr if the working conversions and promotions could not be found.
//
TIntermTyped* TIntermediate::addBinaryMath(TOperator op, TIntermTyped* left, TIntermTyped* right, TSourceLoc loc)
{
    // No operations work on blocks
    if (left->getType().getBasicType() == EbtBlock || right->getType().getBasicType() == EbtBlock)
        return nullptr;

    // Try converting the children's base types to compatible types.
    TIntermTyped* child = addConversion(op, left->getType(), right);
    if (child)
        right = child;
    else {
        child = addConversion(op, right->getType(), left);
        if (child)
            left = child;
        else
            return nullptr;
    }

    // Convert the children's type shape to be compatible.
    addBiShapeConversion(op, left, right);
    if (left == nullptr || right == nullptr)
        return nullptr;

    //
    // Need a new node holding things together.  Make
    // one and promote it to the right type.
    //
    TIntermBinary* node = addBinaryNode(op, left, right, loc);
    if (! promote(node))
        return nullptr;

    node->updatePrecision();

    //
    // If they are both (non-specialization) constants, they must be folded.
    // (Unless it's the sequence (comma) operator, but that's handled in addComma().)
    //
    TIntermConstantUnion *leftTempConstant = node->getLeft()->getAsConstantUnion();
    TIntermConstantUnion *rightTempConstant = node->getRight()->getAsConstantUnion();
    if (leftTempConstant && rightTempConstant) {
        TIntermTyped* folded = leftTempConstant->fold(node->getOp(), rightTempConstant);
        if (folded)
            return folded;
    }

    // If can propagate spec-constantness and if the operation is an allowed
    // specialization-constant operation, make a spec-constant.
    if (specConstantPropagates(*node->getLeft(), *node->getRight()) && isSpecializationOperation(*node))
        node->getWritableType().getQualifier().makeSpecConstant();

    return node;
}

//
// Low level: add binary node (no promotions or other argument modifications)
//
TIntermBinary* TIntermediate::addBinaryNode(TOperator op, TIntermTyped* left, TIntermTyped* right, TSourceLoc loc) const
{
    // build the node
    TIntermBinary* node = new TIntermBinary(op);
    if (loc.line == 0)
        loc = left->getLoc();
    node->setLoc(loc);
    node->setLeft(left);
    node->setRight(right);

    return node;
}

//
// like non-type form, but sets node's type.
//
TIntermBinary* TIntermediate::addBinaryNode(TOperator op, TIntermTyped* left, TIntermTyped* right, TSourceLoc loc, const TType& type) const
{
    TIntermBinary* node = addBinaryNode(op, left, right, loc);
    node->setType(type);
    return node;
}

//
// Low level: add unary node (no promotions or other argument modifications)
//
TIntermUnary* TIntermediate::addUnaryNode(TOperator op, TIntermTyped* child, TSourceLoc loc) const
{
    TIntermUnary* node = new TIntermUnary(op);
    if (loc.line == 0)
        loc = child->getLoc();
    node->setLoc(loc);
    node->setOperand(child);

    return node;
}

//
// like non-type form, but sets node's type.
//
TIntermUnary* TIntermediate::addUnaryNode(TOperator op, TIntermTyped* child, TSourceLoc loc, const TType& type) const
{
    TIntermUnary* node = addUnaryNode(op, child, loc);
    node->setType(type);
    return node;
}

//
// Connect two nodes through an assignment.
//
// Returns the added node.
//
// Returns nullptr if the 'right' type could not be converted to match the 'left' type,
// or the resulting operation cannot be properly promoted.
//
TIntermTyped* TIntermediate::addAssign(TOperator op, TIntermTyped* left, TIntermTyped* right, TSourceLoc loc)
{
    // No block assignment
    if (left->getType().getBasicType() == EbtBlock || right->getType().getBasicType() == EbtBlock)
        return nullptr;

    //
    // Like adding binary math, except the conversion can only go
    // from right to left.
    //

    // convert base types, nullptr return means not possible
    right = addConversion(op, left->getType(), right);
    if (right == nullptr)
        return nullptr;

    // convert shape
    right = addUniShapeConversion(op, left->getType(), right);

    // build the node
    TIntermBinary* node = addBinaryNode(op, left, right, loc);

    if (! promote(node))
        return nullptr;

    node->updatePrecision();

    return node;
}

//
// Connect two nodes through an index operator, where the left node is the base
// of an array or struct, and the right node is a direct or indirect offset.
//
// Returns the added node.
// The caller should set the type of the returned node.
//
TIntermTyped* TIntermediate::addIndex(TOperator op, TIntermTyped* base, TIntermTyped* index, TSourceLoc loc)
{
    // caller should set the type
    return addBinaryNode(op, base, index, loc);
}

//
// Add one node as the parent of another that it operates on.
//
// Returns the added node.
//
TIntermTyped* TIntermediate::addUnaryMath(TOperator op, TIntermTyped* child, TSourceLoc loc)
{
    if (child == 0)
        return nullptr;

    if (child->getType().getBasicType() == EbtBlock)
        return nullptr;

    switch (op) {
    case EOpLogicalNot:
        if (source == EShSourceHlsl) {
            break; // HLSL can promote logical not
        }

        if (child->getType().getBasicType() != EbtBool || child->getType().isMatrix() || child->getType().isArray() || child->getType().isVector()) {
            return nullptr;
        }
        break;

    case EOpPostIncrement:
    case EOpPreIncrement:
    case EOpPostDecrement:
    case EOpPreDecrement:
    case EOpNegative:
        if (child->getType().getBasicType() == EbtStruct || child->getType().isArray())
            return nullptr;
    default: break; // some compilers want this
    }

    //
    // Do we need to promote the operand?
    //
    TBasicType newType = EbtVoid;
    switch (op) {
    case EOpConstructInt:    newType = EbtInt;    break;
    case EOpConstructUint:   newType = EbtUint;   break;
    case EOpConstructInt64:  newType = EbtInt64;  break;
    case EOpConstructUint64: newType = EbtUint64; break;
#ifdef AMD_EXTENSIONS
    case EOpConstructInt16:  newType = EbtInt16;  break;
    case EOpConstructUint16: newType = EbtUint16; break;
#endif
    case EOpConstructBool:   newType = EbtBool;   break;
    case EOpConstructFloat:  newType = EbtFloat;  break;
    case EOpConstructDouble: newType = EbtDouble; break;
#ifdef AMD_EXTENSIONS
    case EOpConstructFloat16: newType = EbtFloat16; break;
#endif
    default: break; // some compilers want this
    }

    if (newType != EbtVoid) {
        child = addConversion(op, TType(newType, EvqTemporary, child->getVectorSize(),
                                                               child->getMatrixCols(),
                                                               child->getMatrixRows(),
                                                               child->isVector()),
                              child);
        if (child == nullptr)
            return nullptr;
    }

    //
    // For constructors, we are now done, it was all in the conversion.
    // TODO: but, did this bypass constant folding?
    //
    switch (op) {
    case EOpConstructInt:
    case EOpConstructUint:
    case EOpConstructInt64:
    case EOpConstructUint64:
#ifdef AMD_EXTENSIONS
    case EOpConstructInt16:
    case EOpConstructUint16:
#endif
    case EOpConstructBool:
    case EOpConstructFloat:
    case EOpConstructDouble:
#ifdef AMD_EXTENSIONS
    case EOpConstructFloat16:
#endif
        return child;
    default: break; // some compilers want this
    }

    //
    // Make a new node for the operator.
    //
    TIntermUnary* node = addUnaryNode(op, child, loc);

    if (! promote(node))
        return nullptr;

    node->updatePrecision();

    // If it's a (non-specialization) constant, it must be folded.
    if (node->getOperand()->getAsConstantUnion())
        return node->getOperand()->getAsConstantUnion()->fold(op, node->getType());

    // If it's a specialization constant, the result is too,
    // if the operation is allowed for specialization constants.
    if (node->getOperand()->getType().getQualifier().isSpecConstant() && isSpecializationOperation(*node))
        node->getWritableType().getQualifier().makeSpecConstant();

    return node;
}

TIntermTyped* TIntermediate::addBuiltInFunctionCall(const TSourceLoc& loc, TOperator op, bool unary, TIntermNode* childNode, const TType& returnType)
{
    if (unary) {
        //
        // Treat it like a unary operator.
        // addUnaryMath() should get the type correct on its own;
        // including constness (which would differ from the prototype).
        //
        TIntermTyped* child = childNode->getAsTyped();
        if (child == nullptr)
            return nullptr;

        if (child->getAsConstantUnion()) {
            TIntermTyped* folded = child->getAsConstantUnion()->fold(op, returnType);
            if (folded)
                return folded;
        }

        return addUnaryNode(op, child, child->getLoc(), returnType);
    } else {
        // setAggregateOperater() calls fold() for constant folding
        TIntermTyped* node = setAggregateOperator(childNode, op, returnType, loc);

        return node;
    }
}

//
// This is the safe way to change the operator on an aggregate, as it
// does lots of error checking and fixing.  Especially for establishing
// a function call's operation on it's set of parameters.  Sequences
// of instructions are also aggregates, but they just directly set
// their operator to EOpSequence.
//
// Returns an aggregate node, which could be the one passed in if
// it was already an aggregate.
//
TIntermTyped* TIntermediate::setAggregateOperator(TIntermNode* node, TOperator op, const TType& type, TSourceLoc loc)
{
    TIntermAggregate* aggNode;

    //
    // Make sure we have an aggregate.  If not turn it into one.
    //
    if (node) {
        aggNode = node->getAsAggregate();
        if (aggNode == nullptr || aggNode->getOp() != EOpNull) {
            //
            // Make an aggregate containing this node.
            //
            aggNode = new TIntermAggregate();
            aggNode->getSequence().push_back(node);
            if (loc.line == 0)
                loc = node->getLoc();
        }
    } else
        aggNode = new TIntermAggregate();

    //
    // Set the operator.
    //
    aggNode->setOperator(op);
    if (loc.line != 0)
        aggNode->setLoc(loc);

    aggNode->setType(type);

    return fold(aggNode);
}

//
// Convert the node's type to the given type, as allowed by the operation involved: 'op'.
// For implicit conversions, 'op' is not the requested conversion, it is the explicit
// operation requiring the implicit conversion.
//
// Returns a node representing the conversion, which could be the same
// node passed in if no conversion was needed.
//
// Generally, this is focused on basic type conversion, not shape conversion.
// See addShapeConversion().
//
// Return nullptr if a conversion can't be done.
//
TIntermTyped* TIntermediate::addConversion(TOperator op, const TType& type, TIntermTyped* node) const
{
    //
    // Does the base type even allow the operation?
    //
    switch (node->getBasicType()) {
    case EbtVoid:
        return nullptr;
    case EbtAtomicUint:
    case EbtSampler:
        // opaque types can be passed to functions
        if (op == EOpFunction)
            break;

        // HLSL can assign samplers directly (no constructor)
        if (source == EShSourceHlsl && node->getBasicType() == EbtSampler)
            break;

        // samplers can get assigned via a sampler constructor
        // (well, not yet, but code in the rest of this function is ready for it)
        if (node->getBasicType() == EbtSampler && op == EOpAssign &&
            node->getAsOperator() != nullptr && node->getAsOperator()->getOp() == EOpConstructTextureSampler)
            break;

        // otherwise, opaque types can't even be operated on, let alone converted
        return nullptr;
    default:
        break;
    }

    // Otherwise, if types are identical, no problem
    if (type == node->getType())
        return node;

    // If one's a structure, then no conversions.
    if (type.isStruct() || node->isStruct())
        return nullptr;

    // If one's an array, then no conversions.
    if (type.isArray() || node->getType().isArray())
        return nullptr;

    // Note: callers are responsible for other aspects of shape,
    // like vector and matrix sizes.

    TBasicType promoteTo;

    switch (op) {
    //
    // Explicit conversions (unary operations)
    //
    case EOpConstructBool:
        promoteTo = EbtBool;
        break;
    case EOpConstructFloat:
        promoteTo = EbtFloat;
        break;
    case EOpConstructDouble:
        promoteTo = EbtDouble;
        break;
#ifdef AMD_EXTENSIONS
    case EOpConstructFloat16:
        promoteTo = EbtFloat16;
        break;
#endif
    case EOpConstructInt:
        promoteTo = EbtInt;
        break;
    case EOpConstructUint:
        promoteTo = EbtUint;
        break;
    case EOpConstructInt64:
        promoteTo = EbtInt64;
        break;
    case EOpConstructUint64:
        promoteTo = EbtUint64;
        break;
#ifdef AMD_EXTENSIONS
    case EOpConstructInt16:
        promoteTo = EbtInt16;
        break;
    case EOpConstructUint16:
        promoteTo = EbtUint16;
        break;
#endif

    //
    // List all the binary ops that can implicitly convert one operand to the other's type;
    // This implements the 'policy' for implicit type conversion.
    //
    case EOpLessThan:
    case EOpGreaterThan:
    case EOpLessThanEqual:
    case EOpGreaterThanEqual:
    case EOpEqual:
    case EOpNotEqual:

    case EOpAdd:
    case EOpSub:
    case EOpMul:
    case EOpDiv:
    case EOpMod:

    case EOpVectorTimesScalar:
    case EOpVectorTimesMatrix:
    case EOpMatrixTimesVector:
    case EOpMatrixTimesScalar:

    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
    case EOpLogicalNot:
    case EOpLogicalAnd:
    case EOpLogicalOr:
    case EOpLogicalXor:

    case EOpFunctionCall:
    case EOpReturn:
    case EOpAssign:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpMulAssign:
    case EOpVectorTimesScalarAssign:
    case EOpMatrixTimesScalarAssign:
    case EOpDivAssign:
    case EOpModAssign:

    case EOpAtan:
    case EOpClamp:
    case EOpCross:
    case EOpDistance:
    case EOpDot:
    case EOpDst:
    case EOpFaceForward:
    case EOpFma:
    case EOpFrexp:
    case EOpLdexp:
    case EOpMix:
    case EOpLit:
    case EOpMax:
    case EOpMin:
    case EOpModf:
    case EOpPow:
    case EOpReflect:
    case EOpRefract:
    case EOpSmoothStep:
    case EOpStep:

    case EOpSequence:
    case EOpConstructStruct:

        if (type.getBasicType() == node->getType().getBasicType())
            return node;

        if (canImplicitlyPromote(node->getType().getBasicType(), type.getBasicType(), op))
            promoteTo = type.getBasicType();
        else
            return nullptr;

        break;

    // Shifts can have mixed types as long as they are integer, without converting.
    // It's the left operand's type that determines the resulting type, so no issue
    // with assign shift ops either.
    case EOpLeftShift:
    case EOpRightShift:
    case EOpLeftShiftAssign:
    case EOpRightShiftAssign:
        if ((type.getBasicType() == EbtInt ||
             type.getBasicType() == EbtUint ||
#ifdef AMD_EXTENSIONS
             type.getBasicType() == EbtInt16 ||
             type.getBasicType() == EbtUint16 ||
#endif
             type.getBasicType() == EbtInt64 ||
             type.getBasicType() == EbtUint64) &&
            (node->getType().getBasicType() == EbtInt ||
             node->getType().getBasicType() == EbtUint ||
#ifdef AMD_EXTENSIONS
             node->getType().getBasicType() == EbtInt16 ||
             node->getType().getBasicType() == EbtUint16 ||
#endif
             node->getType().getBasicType() == EbtInt64 ||
             node->getType().getBasicType() == EbtUint64))

            return node;
        else if (source == EShSourceHlsl && node->getType().getBasicType() == EbtBool) {
            promoteTo = type.getBasicType();
            break;
        } else
            return nullptr;

    default:
        // default is to require a match; all exceptions should have case statements above

        if (type.getBasicType() == node->getType().getBasicType())
            return node;
        else
            return nullptr;
    }

    if (node->getAsConstantUnion())
        return promoteConstantUnion(promoteTo, node->getAsConstantUnion());

    //
    // Add a new newNode for the conversion.
    //
    TIntermUnary* newNode = nullptr;

    TOperator newOp = EOpNull;

    // This is 'mechanism' here, it does any conversion told.  The policy comes
    // from the shader or the above code.
    switch (promoteTo) {
    case EbtDouble:
        switch (node->getBasicType()) {
        case EbtInt:   newOp = EOpConvIntToDouble;   break;
        case EbtUint:  newOp = EOpConvUintToDouble;  break;
        case EbtBool:  newOp = EOpConvBoolToDouble;  break;
        case EbtFloat: newOp = EOpConvFloatToDouble; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToDouble; break;
#endif
        case EbtInt64: newOp = EOpConvInt64ToDouble; break;
        case EbtUint64: newOp = EOpConvUint64ToDouble; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToDouble;  break;
        case EbtUint16: newOp = EOpConvUint16ToDouble; break;
#endif
        default:
            return nullptr;
        }
        break;
    case EbtFloat:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToFloat;    break;
        case EbtUint:   newOp = EOpConvUintToFloat;   break;
        case EbtBool:   newOp = EOpConvBoolToFloat;   break;
        case EbtDouble: newOp = EOpConvDoubleToFloat; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToFloat; break;
#endif
        case EbtInt64:  newOp = EOpConvInt64ToFloat;  break;
        case EbtUint64: newOp = EOpConvUint64ToFloat; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToFloat;  break;
        case EbtUint16: newOp = EOpConvUint16ToFloat; break;
#endif
        default:
            return nullptr;
        }
        break;
#ifdef AMD_EXTENSIONS
    case EbtFloat16:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToFloat16;    break;
        case EbtUint:   newOp = EOpConvUintToFloat16;   break;
        case EbtBool:   newOp = EOpConvBoolToFloat16;   break;
        case EbtFloat:  newOp = EOpConvFloatToFloat16;  break;
        case EbtDouble: newOp = EOpConvDoubleToFloat16; break;
        case EbtInt64:  newOp = EOpConvInt64ToFloat16;  break;
        case EbtUint64: newOp = EOpConvUint64ToFloat16; break;
        case EbtInt16:  newOp = EOpConvInt16ToFloat16;  break;
        case EbtUint16: newOp = EOpConvUint16ToFloat16; break;
        default:
            return nullptr;
        }
        break;
#endif
    case EbtBool:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToBool;    break;
        case EbtUint:   newOp = EOpConvUintToBool;   break;
        case EbtFloat:  newOp = EOpConvFloatToBool;  break;
        case EbtDouble: newOp = EOpConvDoubleToBool; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToBool; break;
#endif
        case EbtInt64:  newOp = EOpConvInt64ToBool;  break;
        case EbtUint64: newOp = EOpConvUint64ToBool; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToBool;  break;
        case EbtUint16: newOp = EOpConvUint16ToBool; break;
#endif
        default:
            return nullptr;
        }
        break;
    case EbtInt:
        switch (node->getBasicType()) {
        case EbtUint:   newOp = EOpConvUintToInt;   break;
        case EbtBool:   newOp = EOpConvBoolToInt;   break;
        case EbtFloat:  newOp = EOpConvFloatToInt;  break;
        case EbtDouble: newOp = EOpConvDoubleToInt; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToInt; break;
#endif
        case EbtInt64:  newOp = EOpConvInt64ToInt;  break;
        case EbtUint64: newOp = EOpConvUint64ToInt; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToInt;  break;
        case EbtUint16: newOp = EOpConvUint16ToInt; break;
#endif
        default:
            return nullptr;
        }
        break;
    case EbtUint:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToUint;    break;
        case EbtBool:   newOp = EOpConvBoolToUint;   break;
        case EbtFloat:  newOp = EOpConvFloatToUint;  break;
        case EbtDouble: newOp = EOpConvDoubleToUint; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToUint; break;
#endif
        case EbtInt64:  newOp = EOpConvInt64ToUint;  break;
        case EbtUint64: newOp = EOpConvUint64ToUint; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToUint;  break;
        case EbtUint16: newOp = EOpConvUint16ToUint; break;
#endif
        default:
            return nullptr;
        }
        break;
    case EbtInt64:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToInt64;    break;
        case EbtUint:   newOp = EOpConvUintToInt64;   break;
        case EbtBool:   newOp = EOpConvBoolToInt64;   break;
        case EbtFloat:  newOp = EOpConvFloatToInt64;  break;
        case EbtDouble: newOp = EOpConvDoubleToInt64; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToInt64; break;
#endif
        case EbtUint64: newOp = EOpConvUint64ToInt64; break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToInt64;  break;
        case EbtUint16: newOp = EOpConvUint16ToInt64; break;
#endif
        default:
            return nullptr;
        }
        break;
    case EbtUint64:
        switch (node->getBasicType()) {
        case EbtInt:    newOp = EOpConvIntToUint64;    break;
        case EbtUint:   newOp = EOpConvUintToUint64;   break;
        case EbtBool:   newOp = EOpConvBoolToUint64;   break;
        case EbtFloat:  newOp = EOpConvFloatToUint64;  break;
        case EbtDouble: newOp = EOpConvDoubleToUint64; break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16: newOp = EOpConvFloat16ToUint64; break;
#endif
        case EbtInt64:  newOp = EOpConvInt64ToUint64;  break;
#ifdef AMD_EXTENSIONS
        case EbtInt16:  newOp = EOpConvInt16ToUint64;  break;
        case EbtUint16: newOp = EOpConvUint16ToUint64; break;
#endif
        default:
            return nullptr;
        }
        break;
#ifdef AMD_EXTENSIONS
    case EbtInt16:
        switch (node->getBasicType()) {
        case EbtInt:     newOp = EOpConvIntToInt16;     break;
        case EbtUint:    newOp = EOpConvUintToInt16;    break;
        case EbtBool:    newOp = EOpConvBoolToInt16;    break;
        case EbtFloat:   newOp = EOpConvFloatToInt16;   break;
        case EbtDouble:  newOp = EOpConvDoubleToInt16;  break;
        case EbtFloat16: newOp = EOpConvFloat16ToInt16; break;
        case EbtInt64:   newOp = EOpConvInt64ToInt16;   break;
        case EbtUint64:  newOp = EOpConvUint64ToInt16;  break;
        case EbtUint16:  newOp = EOpConvUint16ToInt16;  break;
        default:
            return nullptr;
        }
        break;
    case EbtUint16:
        switch (node->getBasicType()) {
        case EbtInt:     newOp = EOpConvIntToUint16;     break;
        case EbtUint:    newOp = EOpConvUintToUint16;    break;
        case EbtBool:    newOp = EOpConvBoolToUint16;    break;
        case EbtFloat:   newOp = EOpConvFloatToUint16;   break;
        case EbtDouble:  newOp = EOpConvDoubleToUint16;  break;
        case EbtFloat16: newOp = EOpConvFloat16ToUint16; break;
        case EbtInt64:   newOp = EOpConvInt64ToUint16;   break;
        case EbtUint64:  newOp = EOpConvUint64ToUint16;  break;
        case EbtInt16:   newOp = EOpConvInt16ToUint16;   break;
        default:
            return nullptr;
        }
        break;
#endif
    default:
        return nullptr;
    }

    TType newType(promoteTo, EvqTemporary, node->getVectorSize(), node->getMatrixCols(), node->getMatrixRows());
    newNode = addUnaryNode(newOp, node, node->getLoc(), newType);

    // TODO: it seems that some unary folding operations should occur here, but are not

    // Propagate specialization-constant-ness, if allowed
    if (node->getType().getQualifier().isSpecConstant() && isSpecializationOperation(*newNode))
        newNode->getWritableType().getQualifier().makeSpecConstant();

    return newNode;
}

// Convert the node's shape of type for the given type, as allowed by the
// operation involved: 'op'.  This is for situations where there is only one
// direction to consider doing the shape conversion.
//
// This implements policy, it call addShapeConversion() for the mechanism.
//
// Generally, the AST represents allowed GLSL shapes, so this isn't needed
// for GLSL.  Bad shapes are caught in conversion or promotion.
//
// Return 'node' if no conversion was done. Promotion handles final shape
// checking.
//
TIntermTyped* TIntermediate::addUniShapeConversion(TOperator op, const TType& type, TIntermTyped* node)
{
    // some source languages don't do this
    switch (source) {
    case EShSourceHlsl:
        break;
    case EShSourceGlsl:
    default:
        return node;
    }

    // some operations don't do this
    switch (op) {
    case EOpFunctionCall:
    case EOpReturn:
        break;

    case EOpMulAssign:
        // want to support vector *= scalar native ops in AST and lower, not smear, similarly for
        // matrix *= scalar, etc.

    case EOpAddAssign:
    case EOpSubAssign:
    case EOpDivAssign:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
    case EOpRightShiftAssign:
    case EOpLeftShiftAssign:
        if (node->getVectorSize() == 1)
            return node;
        break;

    case EOpAssign:
        break;

    case EOpMix:
        break;

    default:
        return node;
    }

    return addShapeConversion(type, node);
}

// Convert the nodes' shapes to be compatible for the operation 'op'.
//
// This implements policy, it call addShapeConversion() for the mechanism.
//
// Generally, the AST represents allowed GLSL shapes, so this isn't needed
// for GLSL.  Bad shapes are caught in conversion or promotion.
//
void TIntermediate::addBiShapeConversion(TOperator op, TIntermTyped*& lhsNode, TIntermTyped*& rhsNode)
{
    // some source languages don't do this
    switch (source) {
    case EShSourceHlsl:
        break;
    case EShSourceGlsl:
    default:
        return;
    }

    // some operations don't do this
    // 'break' will mean attempt bidirectional conversion
    switch (op) {
    case EOpMulAssign:
    case EOpAssign:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpDivAssign:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
    case EOpRightShiftAssign:
    case EOpLeftShiftAssign:
        // switch to unidirectional conversion (the lhs can't change)
        rhsNode = addUniShapeConversion(op, lhsNode->getType(), rhsNode);
        return;

    case EOpAdd:
    case EOpSub:
    case EOpMul:
    case EOpDiv:
        // want to support vector * scalar native ops in AST and lower, not smear, similarly for
        // matrix * vector, etc.
        if (lhsNode->getVectorSize() == 1 || rhsNode->getVectorSize() == 1)
            return;
        break;

    case EOpRightShift:
    case EOpLeftShift:
        // can natively support the right operand being a scalar and the left a vector,
        // but not the reverse
        if (rhsNode->getVectorSize() == 1)
            return;
        break;

    case EOpLessThan:
    case EOpGreaterThan:
    case EOpLessThanEqual:
    case EOpGreaterThanEqual:

    case EOpEqual:
    case EOpNotEqual:

    case EOpLogicalAnd:
    case EOpLogicalOr:
    case EOpLogicalXor:

    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:

    case EOpMix:
        break;

    default:
        return;
    }

    // Do bidirectional conversions
    if (lhsNode->getType().isScalarOrVec1() || rhsNode->getType().isScalarOrVec1()) {
        if (lhsNode->getType().isScalarOrVec1())
            lhsNode = addShapeConversion(rhsNode->getType(), lhsNode);
        else
            rhsNode = addShapeConversion(lhsNode->getType(), rhsNode);
    }
    lhsNode = addShapeConversion(rhsNode->getType(), lhsNode);
    rhsNode = addShapeConversion(lhsNode->getType(), rhsNode);
}

// Convert the node's shape of type for the given type. It's not necessarily
// an error if they are different and not converted, as some operations accept
// mixed types.  Promotion will do final shape checking.
//
// If there is a chance of two nodes, with conversions possible in each direction,
// the policy for what to ask for must be in the caller; this will do what is asked.
//
// Return 'node' if no conversion was done. Promotion handles final shape
// checking.
//
TIntermTyped* TIntermediate::addShapeConversion(const TType& type, TIntermTyped* node)
{
    // no conversion needed
    if (node->getType() == type)
        return node;

    // structures and arrays don't change shape, either to or from
    if (node->getType().isStruct() || node->getType().isArray() ||
        type.isStruct() || type.isArray())
        return node;

    // The new node that handles the conversion
    TOperator constructorOp = mapTypeToConstructorOp(type);

    // HLSL has custom semantics for scalar->mat shape conversions.
    if (source == EShSourceHlsl) {
        if (node->getType().isScalarOrVec1() && type.isMatrix()) {

            // HLSL semantics: the scalar (or vec1) is replicated to every component of the matrix.  Left to its
            // own devices, the constructor from a scalar would populate the diagonal.  This forces replication
            // to every matrix element.

            // Note that if the node is complex (e.g, a function call), we don't want to duplicate it here
            // repeatedly, so we copy it to a temp, then use the temp.
            const int matSize = type.getMatrixRows() * type.getMatrixCols();
            TIntermAggregate* rhsAggregate = new TIntermAggregate();

            const bool isSimple = (node->getAsSymbolNode() != nullptr) || (node->getAsConstantUnion() != nullptr);

            if (!isSimple) {
                assert(0); // TODO: use node replicator service when available.
            }
            
            for (int x=0; x<matSize; ++x)
                rhsAggregate->getSequence().push_back(node);

            return setAggregateOperator(rhsAggregate, constructorOp, type, node->getLoc());
        }
    }

    // scalar -> vector or vec1 -> vector or
    // vector -> scalar or
    // bigger vector -> smaller vector
    if ((node->getType().isScalarOrVec1() && type.isVector()) ||
        (node->getType().isVector() && type.isScalar()) ||
        (node->isVector() && type.isVector() && node->getVectorSize() > type.getVectorSize()))
        return setAggregateOperator(makeAggregate(node), constructorOp, type, node->getLoc());

    return node;
}

//
// See if the 'from' type is allowed to be implicitly converted to the
// 'to' type.  This is not about vector/array/struct, only about basic type.
//
bool TIntermediate::canImplicitlyPromote(TBasicType from, TBasicType to, TOperator op) const
{
    if (profile == EEsProfile || version == 110)
        return false;

    if (from == to)
        return true;

    // TODO: Move more policies into language-specific handlers.
    // Some languages allow more general (or potentially, more specific) conversions under some conditions.
    if (source == EShSourceHlsl) {
        const bool fromConvertable = (from == EbtFloat || from == EbtDouble || from == EbtInt || from == EbtUint || from == EbtBool);
        const bool toConvertable = (to == EbtFloat || to == EbtDouble || to == EbtInt || to == EbtUint || to == EbtBool);

        if (fromConvertable && toConvertable) {
            switch (op) {
            case EOpAndAssign:               // assignments can perform arbitrary conversions
            case EOpInclusiveOrAssign:       // ...
            case EOpExclusiveOrAssign:       // ...
            case EOpAssign:                  // ...
            case EOpAddAssign:               // ...
            case EOpSubAssign:               // ...
            case EOpMulAssign:               // ...
            case EOpVectorTimesScalarAssign: // ...
            case EOpMatrixTimesScalarAssign: // ...
            case EOpDivAssign:               // ...
            case EOpModAssign:               // ...
            case EOpReturn:                  // function returns can also perform arbitrary conversions
            case EOpFunctionCall:            // conversion of a calling parameter
            case EOpLogicalNot:
            case EOpLogicalAnd:
            case EOpLogicalOr:
            case EOpLogicalXor:
            case EOpConstructStruct:
                return true;
            default:
                break;
            }
        }
    }

    switch (to) {
    case EbtDouble:
        switch (from) {
        case EbtInt:
        case EbtUint:
        case EbtInt64:
        case EbtUint64:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
        case EbtUint16:
#endif
        case EbtFloat:
        case EbtDouble:
#ifdef AMD_EXTENSIONS
        case EbtFloat16:
#endif
            return true;
        default:
            return false;
        }
    case EbtFloat:
        switch (from) {
        case EbtInt:
        case EbtUint:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
        case EbtUint16:
#endif
        case EbtFloat:
#ifdef AMD_EXTENSIONS
        case EbtFloat16:
#endif
            return true;
        case EbtBool:
            return (source == EShSourceHlsl);
        default:
            return false;
        }
    case EbtUint:
        switch (from) {
        case EbtInt:
            return version >= 400 || (source == EShSourceHlsl);
        case EbtUint:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
        case EbtUint16:
#endif
            return true;
        case EbtBool:
            return (source == EShSourceHlsl);
        default:
            return false;
        }
    case EbtInt:
        switch (from) {
        case EbtInt:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
#endif
            return true;
        case EbtBool:
            return (source == EShSourceHlsl);
        default:
            return false;
        }
    case EbtUint64:
        switch (from) {
        case EbtInt:
        case EbtUint:
        case EbtInt64:
        case EbtUint64:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
        case EbtUint16:
#endif
            return true;
        default:
            return false;
        }
    case EbtInt64:
        switch (from) {
        case EbtInt:
        case EbtInt64:
#ifdef AMD_EXTENSIONS
        case EbtInt16:
#endif
            return true;
        default:
            return false;
        }
#ifdef AMD_EXTENSIONS
    case EbtFloat16:
        switch (from) {
        case EbtInt16:
        case EbtUint16:
        case EbtFloat16:
            return true;
        default:
            return false;
        }
    case EbtUint16:
        switch (from) {
        case EbtInt16:
        case EbtUint16:
            return true;
        default:
            return false;
        }
#endif
    default:
        return false;
    }
}

//
// Given a type, find what operation would fully construct it.
//
TOperator TIntermediate::mapTypeToConstructorOp(const TType& type) const
{
    TOperator op = EOpNull;

    switch (type.getBasicType()) {
    case EbtStruct:
        op = EOpConstructStruct;
        break;
    case EbtSampler:
        if (type.getSampler().combined)
            op = EOpConstructTextureSampler;
        break;
    case EbtFloat:
        if (type.isMatrix()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructMat2x2; break;
                case 3: op = EOpConstructMat2x3; break;
                case 4: op = EOpConstructMat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructMat3x2; break;
                case 3: op = EOpConstructMat3x3; break;
                case 4: op = EOpConstructMat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructMat4x2; break;
                case 3: op = EOpConstructMat4x3; break;
                case 4: op = EOpConstructMat4x4; break;
                default: break; // some compilers want this
                }
                break;
            default: break; // some compilers want this
            }
        } else {
            switch(type.getVectorSize()) {
            case 1: op = EOpConstructFloat; break;
            case 2: op = EOpConstructVec2;  break;
            case 3: op = EOpConstructVec3;  break;
            case 4: op = EOpConstructVec4;  break;
            default: break; // some compilers want this
            }
        }
        break;
    case EbtDouble:
        if (type.getMatrixCols()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructDMat2x2; break;
                case 3: op = EOpConstructDMat2x3; break;
                case 4: op = EOpConstructDMat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructDMat3x2; break;
                case 3: op = EOpConstructDMat3x3; break;
                case 4: op = EOpConstructDMat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructDMat4x2; break;
                case 3: op = EOpConstructDMat4x3; break;
                case 4: op = EOpConstructDMat4x4; break;
                default: break; // some compilers want this
                }
                break;
            }
        } else {
            switch(type.getVectorSize()) {
            case 1: op = EOpConstructDouble; break;
            case 2: op = EOpConstructDVec2;  break;
            case 3: op = EOpConstructDVec3;  break;
            case 4: op = EOpConstructDVec4;  break;
            default: break; // some compilers want this
            }
        }
        break;
#ifdef AMD_EXTENSIONS
    case EbtFloat16:
        if (type.getMatrixCols()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructF16Mat2x2; break;
                case 3: op = EOpConstructF16Mat2x3; break;
                case 4: op = EOpConstructF16Mat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructF16Mat3x2; break;
                case 3: op = EOpConstructF16Mat3x3; break;
                case 4: op = EOpConstructF16Mat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructF16Mat4x2; break;
                case 3: op = EOpConstructF16Mat4x3; break;
                case 4: op = EOpConstructF16Mat4x4; break;
                default: break; // some compilers want this
                }
                break;
            }
        }
        else {
            switch (type.getVectorSize()) {
            case 1: op = EOpConstructFloat16;  break;
            case 2: op = EOpConstructF16Vec2;  break;
            case 3: op = EOpConstructF16Vec3;  break;
            case 4: op = EOpConstructF16Vec4;  break;
            default: break; // some compilers want this
            }
        }
        break;
#endif
    case EbtInt:
        if (type.getMatrixCols()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructIMat2x2; break;
                case 3: op = EOpConstructIMat2x3; break;
                case 4: op = EOpConstructIMat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructIMat3x2; break;
                case 3: op = EOpConstructIMat3x3; break;
                case 4: op = EOpConstructIMat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructIMat4x2; break;
                case 3: op = EOpConstructIMat4x3; break;
                case 4: op = EOpConstructIMat4x4; break;
                default: break; // some compilers want this
                }
                break;
            }
        } else {
            switch(type.getVectorSize()) {
            case 1: op = EOpConstructInt;   break;
            case 2: op = EOpConstructIVec2; break;
            case 3: op = EOpConstructIVec3; break;
            case 4: op = EOpConstructIVec4; break;
            default: break; // some compilers want this
            }
        }
        break;
    case EbtUint:
        if (type.getMatrixCols()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructUMat2x2; break;
                case 3: op = EOpConstructUMat2x3; break;
                case 4: op = EOpConstructUMat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructUMat3x2; break;
                case 3: op = EOpConstructUMat3x3; break;
                case 4: op = EOpConstructUMat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructUMat4x2; break;
                case 3: op = EOpConstructUMat4x3; break;
                case 4: op = EOpConstructUMat4x4; break;
                default: break; // some compilers want this
                }
                break;
            }
        } else {
            switch(type.getVectorSize()) {
            case 1: op = EOpConstructUint;  break;
            case 2: op = EOpConstructUVec2; break;
            case 3: op = EOpConstructUVec3; break;
            case 4: op = EOpConstructUVec4; break;
            default: break; // some compilers want this
            }
        }
        break;
    case EbtInt64:
        switch(type.getVectorSize()) {
        case 1: op = EOpConstructInt64;   break;
        case 2: op = EOpConstructI64Vec2; break;
        case 3: op = EOpConstructI64Vec3; break;
        case 4: op = EOpConstructI64Vec4; break;
        default: break; // some compilers want this
        }
        break;
    case EbtUint64:
        switch(type.getVectorSize()) {
        case 1: op = EOpConstructUint64;  break;
        case 2: op = EOpConstructU64Vec2; break;
        case 3: op = EOpConstructU64Vec3; break;
        case 4: op = EOpConstructU64Vec4; break;
        default: break; // some compilers want this
        }
        break;
#ifdef AMD_EXTENSIONS
    case EbtInt16:
        switch(type.getVectorSize()) {
        case 1: op = EOpConstructInt16;   break;
        case 2: op = EOpConstructI16Vec2; break;
        case 3: op = EOpConstructI16Vec3; break;
        case 4: op = EOpConstructI16Vec4; break;
        default: break; // some compilers want this
        }
        break;
    case EbtUint16:
        switch(type.getVectorSize()) {
        case 1: op = EOpConstructUint16;  break;
        case 2: op = EOpConstructU16Vec2; break;
        case 3: op = EOpConstructU16Vec3; break;
        case 4: op = EOpConstructU16Vec4; break;
        default: break; // some compilers want this
        }
        break;
#endif
    case EbtBool:
        if (type.getMatrixCols()) {
            switch (type.getMatrixCols()) {
            case 2:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructBMat2x2; break;
                case 3: op = EOpConstructBMat2x3; break;
                case 4: op = EOpConstructBMat2x4; break;
                default: break; // some compilers want this
                }
                break;
            case 3:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructBMat3x2; break;
                case 3: op = EOpConstructBMat3x3; break;
                case 4: op = EOpConstructBMat3x4; break;
                default: break; // some compilers want this
                }
                break;
            case 4:
                switch (type.getMatrixRows()) {
                case 2: op = EOpConstructBMat4x2; break;
                case 3: op = EOpConstructBMat4x3; break;
                case 4: op = EOpConstructBMat4x4; break;
                default: break; // some compilers want this
                }
                break;
            }
        } else {
            switch(type.getVectorSize()) {
            case 1:  op = EOpConstructBool;  break;
            case 2:  op = EOpConstructBVec2; break;
            case 3:  op = EOpConstructBVec3; break;
            case 4:  op = EOpConstructBVec4; break;
            default: break; // some compilers want this
            }
        }
        break;
    default:
        break;
    }

    return op;
}

//
// Safe way to combine two nodes into an aggregate.  Works with null pointers,
// a node that's not a aggregate yet, etc.
//
// Returns the resulting aggregate, unless nullptr was passed in for
// both existing nodes.
//
TIntermAggregate* TIntermediate::growAggregate(TIntermNode* left, TIntermNode* right)
{
    if (left == nullptr && right == nullptr)
        return nullptr;

    TIntermAggregate* aggNode = nullptr;
    if (left != nullptr)
        aggNode = left->getAsAggregate();
    if (aggNode == nullptr || aggNode->getOp() != EOpNull) {
        aggNode = new TIntermAggregate;
        if (left != nullptr)
            aggNode->getSequence().push_back(left);
    }

    if (right != nullptr)
        aggNode->getSequence().push_back(right);

    return aggNode;
}

TIntermAggregate* TIntermediate::growAggregate(TIntermNode* left, TIntermNode* right, const TSourceLoc& loc)
{
    TIntermAggregate* aggNode = growAggregate(left, right);
    if (aggNode)
        aggNode->setLoc(loc);

    return aggNode;
}

//
// Turn an existing node into an aggregate.
//
// Returns an aggregate, unless nullptr was passed in for the existing node.
//
TIntermAggregate* TIntermediate::makeAggregate(TIntermNode* node)
{
    if (node == nullptr)
        return nullptr;

    TIntermAggregate* aggNode = new TIntermAggregate;
    aggNode->getSequence().push_back(node);
    aggNode->setLoc(node->getLoc());

    return aggNode;
}

TIntermAggregate* TIntermediate::makeAggregate(TIntermNode* node, const TSourceLoc& loc)
{
    if (node == nullptr)
        return nullptr;

    TIntermAggregate* aggNode = new TIntermAggregate;
    aggNode->getSequence().push_back(node);
    aggNode->setLoc(loc);

    return aggNode;
}

//
// Make an aggregate with an empty sequence.
//
TIntermAggregate* TIntermediate::makeAggregate(const TSourceLoc& loc)
{
    TIntermAggregate* aggNode = new TIntermAggregate;
    aggNode->setLoc(loc);

    return aggNode;
}

//
// For "if" test nodes.  There are three children; a condition,
// a true path, and a false path.  The two paths are in the
// nodePair.
//
// Returns the selection node created.
//
TIntermTyped* TIntermediate::addSelection(TIntermTyped* cond, TIntermNodePair nodePair, const TSourceLoc& loc)
{
    //
    // Don't prune the false path for compile-time constants; it's needed
    // for static access analysis.
    //

    TIntermSelection* node = new TIntermSelection(cond, nodePair.node1, nodePair.node2);
    node->setLoc(loc);

    return node;
}

TIntermTyped* TIntermediate::addComma(TIntermTyped* left, TIntermTyped* right, const TSourceLoc& loc)
{
    // However, the lowest precedence operators of the sequence operator ( , ) and the assignment operators
    // ... are not included in the operators that can create a constant expression.
    //
    // if (left->getType().getQualifier().storage == EvqConst &&
    //    right->getType().getQualifier().storage == EvqConst) {

    //    return right;
    //}

    TIntermTyped *commaAggregate = growAggregate(left, right, loc);
    commaAggregate->getAsAggregate()->setOperator(EOpComma);
    commaAggregate->setType(right->getType());
    commaAggregate->getWritableType().getQualifier().makeTemporary();

    return commaAggregate;
}

TIntermTyped* TIntermediate::addMethod(TIntermTyped* object, const TType& type, const TString* name, const TSourceLoc& loc)
{
    TIntermMethod* method = new TIntermMethod(object, type, *name);
    method->setLoc(loc);

    return method;
}

//
// For "?:" test nodes.  There are three children; a condition,
// a true path, and a false path.  The two paths are specified
// as separate parameters. For vector 'cond', the true and false
// are not paths, but vectors to mix.
//
// Specialization constant operations include
//     - The ternary operator ( ? : )
//
// Returns the selection node created, or nullptr if one could not be.
//
TIntermTyped* TIntermediate::addSelection(TIntermTyped* cond, TIntermTyped* trueBlock, TIntermTyped* falseBlock, const TSourceLoc& loc)
{
    // If it's void, go to the if-then-else selection()
    if (trueBlock->getBasicType() == EbtVoid && falseBlock->getBasicType() == EbtVoid) {
        TIntermNodePair pair = { trueBlock, falseBlock };
        return addSelection(cond, pair, loc);
    }

    //
    // Get compatible types.
    //
    TIntermTyped* child = addConversion(EOpSequence, trueBlock->getType(), falseBlock);
    if (child)
        falseBlock = child;
    else {
        child = addConversion(EOpSequence, falseBlock->getType(), trueBlock);
        if (child)
            trueBlock = child;
        else
            return nullptr;
    }

    // Handle a vector condition as a mix
    if (!cond->getType().isScalarOrVec1()) {
        TType targetVectorType(trueBlock->getType().getBasicType(), EvqTemporary,
                               cond->getType().getVectorSize());
        // smear true/false operands as needed
        trueBlock = addUniShapeConversion(EOpMix, targetVectorType, trueBlock);
        falseBlock = addUniShapeConversion(EOpMix, targetVectorType, falseBlock);

        // After conversion, types have to match.
        if (falseBlock->getType() != trueBlock->getType())
            return nullptr;

        // make the mix operation
        TIntermAggregate* mix = makeAggregate(loc);
        mix = growAggregate(mix, falseBlock);
        mix = growAggregate(mix, trueBlock);
        mix = growAggregate(mix, cond);
        mix->setType(targetVectorType);
        mix->setOp(EOpMix);

        return mix;
    }

    // Now have a scalar condition...

    // Convert true and false expressions to matching types
    addBiShapeConversion(EOpMix, trueBlock, falseBlock);

    // After conversion, types have to match.
    if (falseBlock->getType() != trueBlock->getType())
        return nullptr;

    // Eliminate the selection when the condition is a scalar and all operands are constant.
    if (cond->getAsConstantUnion() && trueBlock->getAsConstantUnion() && falseBlock->getAsConstantUnion()) {
        if (cond->getAsConstantUnion()->getConstArray()[0].getBConst())
            return trueBlock;
        else
            return falseBlock;
    }

    //
    // Make a selection node.
    //
    TIntermSelection* node = new TIntermSelection(cond, trueBlock, falseBlock, trueBlock->getType());
    node->setLoc(loc);
    node->getQualifier().precision = std::max(trueBlock->getQualifier().precision, falseBlock->getQualifier().precision);

    if ((cond->getQualifier().isConstant() && specConstantPropagates(*trueBlock, *falseBlock)) ||
        (cond->getQualifier().isSpecConstant() && trueBlock->getQualifier().isConstant() &&
                                                 falseBlock->getQualifier().isConstant()))
        node->getQualifier().makeSpecConstant();
    else
        node->getQualifier().makeTemporary();

    return node;
}

//
// Constant terminal nodes.  Has a union that contains bool, float or int constants
//
// Returns the constant union node created.
//

TIntermConstantUnion* TIntermediate::addConstantUnion(const TConstUnionArray& unionArray, const TType& t, const TSourceLoc& loc, bool literal) const
{
    TIntermConstantUnion* node = new TIntermConstantUnion(unionArray, t);
    node->getQualifier().storage = EvqConst;
    node->setLoc(loc);
    if (literal)
        node->setLiteral();

    return node;
}

TIntermConstantUnion* TIntermediate::addConstantUnion(int i, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setIConst(i);

    return addConstantUnion(unionArray, TType(EbtInt, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(unsigned int u, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setUConst(u);

    return addConstantUnion(unionArray, TType(EbtUint, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(long long i64, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setI64Const(i64);

    return addConstantUnion(unionArray, TType(EbtInt64, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(unsigned long long u64, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setU64Const(u64);

    return addConstantUnion(unionArray, TType(EbtUint64, EvqConst), loc, literal);
}

#ifdef AMD_EXTENSIONS
TIntermConstantUnion* TIntermediate::addConstantUnion(short i16, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setIConst(i16);

    return addConstantUnion(unionArray, TType(EbtInt16, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(unsigned short u16, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setUConst(u16);

    return addConstantUnion(unionArray, TType(EbtUint16, EvqConst), loc, literal);
}
#endif

TIntermConstantUnion* TIntermediate::addConstantUnion(bool b, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setBConst(b);

    return addConstantUnion(unionArray, TType(EbtBool, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(double d, TBasicType baseType, const TSourceLoc& loc, bool literal) const
{
#ifdef AMD_EXTENSIONS
    assert(baseType == EbtFloat || baseType == EbtDouble || baseType == EbtFloat16);
#else
    assert(baseType == EbtFloat || baseType == EbtDouble);
#endif

    TConstUnionArray unionArray(1);
    unionArray[0].setDConst(d);

    return addConstantUnion(unionArray, TType(baseType, EvqConst), loc, literal);
}

TIntermConstantUnion* TIntermediate::addConstantUnion(const TString* s, const TSourceLoc& loc, bool literal) const
{
    TConstUnionArray unionArray(1);
    unionArray[0].setSConst(s);

    return addConstantUnion(unionArray, TType(EbtString, EvqConst), loc, literal);
}

// Put vector swizzle selectors onto the given sequence
void TIntermediate::pushSelector(TIntermSequence& sequence, const TVectorSelector& selector, const TSourceLoc& loc)
{
    TIntermConstantUnion* constIntNode = addConstantUnion(selector, loc);
    sequence.push_back(constIntNode);
}

// Put matrix swizzle selectors onto the given sequence
void TIntermediate::pushSelector(TIntermSequence& sequence, const TMatrixSelector& selector, const TSourceLoc& loc)
{
    TIntermConstantUnion* constIntNode = addConstantUnion(selector.coord1, loc);
    sequence.push_back(constIntNode);
    constIntNode = addConstantUnion(selector.coord2, loc);
    sequence.push_back(constIntNode);
}

// Make an aggregate node that has a sequence of all selectors.
template TIntermTyped* TIntermediate::addSwizzle<TVectorSelector>(TSwizzleSelectors<TVectorSelector>& selector, const TSourceLoc& loc);
template TIntermTyped* TIntermediate::addSwizzle<TMatrixSelector>(TSwizzleSelectors<TMatrixSelector>& selector, const TSourceLoc& loc);
template<typename selectorType>
TIntermTyped* TIntermediate::addSwizzle(TSwizzleSelectors<selectorType>& selector, const TSourceLoc& loc)
{
    TIntermAggregate* node = new TIntermAggregate(EOpSequence);

    node->setLoc(loc);
    TIntermSequence &sequenceVector = node->getSequence();

    for (int i = 0; i < selector.size(); i++)
        pushSelector(sequenceVector, selector[i], loc);

    return node;
}

//
// Follow the left branches down to the root of an l-value
// expression (just "." and []).
//
// Return the base of the l-value (where following indexing quits working).
// Return nullptr if a chain following dereferences cannot be followed.
//
// 'swizzleOkay' says whether or not it is okay to consider a swizzle
// a valid part of the dereference chain.
//
const TIntermTyped* TIntermediate::findLValueBase(const TIntermTyped* node, bool swizzleOkay)
{
    do {
        const TIntermBinary* binary = node->getAsBinaryNode();
        if (binary == nullptr)
            return node;
        TOperator op = binary->getOp();
        if (op != EOpIndexDirect && op != EOpIndexIndirect && op != EOpIndexDirectStruct && op != EOpVectorSwizzle && op != EOpMatrixSwizzle)
            return nullptr;
        if (! swizzleOkay) {
            if (op == EOpVectorSwizzle || op == EOpMatrixSwizzle)
                return nullptr;
            if ((op == EOpIndexDirect || op == EOpIndexIndirect) &&
                (binary->getLeft()->getType().isVector() || binary->getLeft()->getType().isScalar()) &&
                ! binary->getLeft()->getType().isArray())
                return nullptr;
        }
        node = node->getAsBinaryNode()->getLeft();
    } while (true);
}

//
// Create while and do-while loop nodes.
//
TIntermLoop* TIntermediate::addLoop(TIntermNode* body, TIntermTyped* test, TIntermTyped* terminal, bool testFirst, const TSourceLoc& loc, TLoopControl control)
{
    TIntermLoop* node = new TIntermLoop(body, test, terminal, testFirst);
    node->setLoc(loc);
    node->setLoopControl(control);

    return node;
}

//
// Create a for-loop sequence.
//
TIntermAggregate* TIntermediate::addForLoop(TIntermNode* body, TIntermNode* initializer, TIntermTyped* test, TIntermTyped* terminal, bool testFirst, const TSourceLoc& loc, TLoopControl control)
{
    TIntermLoop* node = new TIntermLoop(body, test, terminal, testFirst);
    node->setLoc(loc);
    node->setLoopControl(control);

    // make a sequence of the initializer and statement, but try to reuse the
    // aggregate already created for whatever is in the initializer, if there is one
    TIntermAggregate* loopSequence = (initializer == nullptr ||
                                      initializer->getAsAggregate() == nullptr) ? makeAggregate(initializer, loc)
                                                                                : initializer->getAsAggregate();
    if (loopSequence != nullptr && loopSequence->getOp() == EOpSequence)
        loopSequence->setOp(EOpNull);
    loopSequence = growAggregate(loopSequence, node);
    loopSequence->setOperator(EOpSequence);

    return loopSequence;
}

//
// Add branches.
//
TIntermBranch* TIntermediate::addBranch(TOperator branchOp, const TSourceLoc& loc)
{
    return addBranch(branchOp, nullptr, loc);
}

TIntermBranch* TIntermediate::addBranch(TOperator branchOp, TIntermTyped* expression, const TSourceLoc& loc)
{
    TIntermBranch* node = new TIntermBranch(branchOp, expression);
    node->setLoc(loc);

    return node;
}

//
// This is to be executed after the final root is put on top by the parsing
// process.
//
bool TIntermediate::postProcess(TIntermNode* root, EShLanguage /*language*/)
{
    if (root == nullptr)
        return true;

    // Finish off the top-level sequence
    TIntermAggregate* aggRoot = root->getAsAggregate();
    if (aggRoot && aggRoot->getOp() == EOpNull)
        aggRoot->setOperator(EOpSequence);

    // Propagate 'noContraction' label in backward from 'precise' variables.
    glslang::PropagateNoContraction(*this);

    switch (textureSamplerTransformMode) {
    case EShTexSampTransKeep:
        break;
    case EShTexSampTransUpgradeTextureRemoveSampler:
        performTextureUpgradeAndSamplerRemovalTransformation(root);
        break;
    }

    return true;
}

void TIntermediate::addSymbolLinkageNodes(TIntermAggregate*& linkage, EShLanguage language, TSymbolTable& symbolTable)
{
    // Add top-level nodes for declarations that must be checked cross
    // compilation unit by a linker, yet might not have been referenced
    // by the AST.
    //
    // Almost entirely, translation of symbols is driven by what's present
    // in the AST traversal, not by translating the symbol table.
    //
    // However, there are some special cases:
    //  - From the specification: "Special built-in inputs gl_VertexID and
    //    gl_InstanceID are also considered active vertex attributes."
    //  - Linker-based type mismatch error reporting needs to see all
    //    uniforms/ins/outs variables and blocks.
    //  - ftransform() can make gl_Vertex and gl_ModelViewProjectionMatrix active.
    //

    // if (ftransformUsed) {
        // TODO: 1.1 lowering functionality: track ftransform() usage
    //    addSymbolLinkageNode(root, symbolTable, "gl_Vertex");
    //    addSymbolLinkageNode(root, symbolTable, "gl_ModelViewProjectionMatrix");
    //}

    if (language == EShLangVertex) {
        // the names won't be found in the symbol table unless the versions are right,
        // so version logic does not need to be repeated here
        addSymbolLinkageNode(linkage, symbolTable, "gl_VertexID");
        addSymbolLinkageNode(linkage, symbolTable, "gl_InstanceID");
    }

    // Add a child to the root node for the linker objects
    linkage->setOperator(EOpLinkerObjects);
    treeRoot = growAggregate(treeRoot, linkage);
}

//
// Add the given name or symbol to the list of nodes at the end of the tree used
// for link-time checking and external linkage.
//

void TIntermediate::addSymbolLinkageNode(TIntermAggregate*& linkage, TSymbolTable& symbolTable, const TString& name)
{
    TSymbol* symbol = symbolTable.find(name);
    if (symbol)
        addSymbolLinkageNode(linkage, *symbol->getAsVariable());
}

void TIntermediate::addSymbolLinkageNode(TIntermAggregate*& linkage, const TSymbol& symbol)
{
    const TVariable* variable = symbol.getAsVariable();
    if (! variable) {
        // This must be a member of an anonymous block, and we need to add the whole block
        const TAnonMember* anon = symbol.getAsAnonMember();
        variable = &anon->getAnonContainer();
    }
    TIntermSymbol* node = addSymbol(*variable);
    linkage = growAggregate(linkage, node);
}

//
// Add a caller->callee relationship to the call graph.
// Assumes the strings are unique per signature.
//
void TIntermediate::addToCallGraph(TInfoSink& /*infoSink*/, const TString& caller, const TString& callee)
{
    // Duplicates are okay, but faster to not keep them, and they come grouped by caller,
    // as long as new ones are push on the same end we check on for duplicates
    for (TGraph::const_iterator call = callGraph.begin(); call != callGraph.end(); ++call) {
        if (call->caller != caller)
            break;
        if (call->callee == callee)
            return;
    }

    callGraph.push_front(TCall(caller, callee));
}

//
// This deletes the tree.
//
void TIntermediate::removeTree()
{
    if (treeRoot)
        RemoveAllTreeNodes(treeRoot);
}

//
// Implement the part of KHR_vulkan_glsl that lists the set of operations
// that can result in a specialization constant operation.
//
// "5.x Specialization Constant Operations"
//
//    Only some operations discussed in this section may be applied to a
//    specialization constant and still yield a result that is as
//    specialization constant.  The operations allowed are listed below.
//    When a specialization constant is operated on with one of these
//    operators and with another constant or specialization constant, the
//    result is implicitly a specialization constant.
//
//     - int(), uint(), and bool() constructors for type conversions
//       from any of the following types to any of the following types:
//         * int
//         * uint
//         * bool
//     - vector versions of the above conversion constructors
//     - allowed implicit conversions of the above
//     - swizzles (e.g., foo.yx)
//     - The following when applied to integer or unsigned integer types:
//         * unary negative ( - )
//         * binary operations ( + , - , * , / , % )
//         * shift ( <<, >> )
//         * bitwise operations ( & , | , ^ )
//     - The following when applied to integer or unsigned integer scalar types:
//         * comparison ( == , != , > , >= , < , <= )
//     - The following when applied to the Boolean scalar type:
//         * not ( ! )
//         * logical operations ( && , || , ^^ )
//         * comparison ( == , != )"
//
// This function just handles binary and unary nodes.  Construction
// rules are handled in construction paths that are not covered by the unary
// and binary paths, while required conversions will still show up here
// as unary converters in the from a construction operator.
//
bool TIntermediate::isSpecializationOperation(const TIntermOperator& node) const
{
    // The operations resulting in floating point are quite limited
    // (However, some floating-point operations result in bool, like ">",
    // so are handled later.)
    if (node.getType().isFloatingDomain()) {
        switch (node.getOp()) {
        case EOpIndexDirect:
        case EOpIndexIndirect:
        case EOpIndexDirectStruct:
        case EOpVectorSwizzle:
        case EOpConvFloatToDouble:
        case EOpConvDoubleToFloat:
#ifdef AMD_EXTENSIONS
        case EOpConvFloat16ToFloat:
        case EOpConvFloatToFloat16:
        case EOpConvFloat16ToDouble:
        case EOpConvDoubleToFloat16:
#endif
            return true;
        default:
            return false;
        }
    }

    // Check for floating-point arguments
    if (const TIntermBinary* bin = node.getAsBinaryNode())
        if (bin->getLeft() ->getType().isFloatingDomain() ||
            bin->getRight()->getType().isFloatingDomain())
            return false;

    // So, for now, we can assume everything left is non-floating-point...

    // Now check for integer/bool-based operations
    switch (node.getOp()) {

    // dereference/swizzle
    case EOpIndexDirect:
    case EOpIndexIndirect:
    case EOpIndexDirectStruct:
    case EOpVectorSwizzle:

    // conversion constructors
    case EOpConvIntToBool:
    case EOpConvUintToBool:
    case EOpConvUintToInt:
    case EOpConvBoolToInt:
    case EOpConvIntToUint:
    case EOpConvBoolToUint:
    case EOpConvInt64ToBool:
    case EOpConvBoolToInt64:
    case EOpConvUint64ToBool:
    case EOpConvBoolToUint64:
    case EOpConvInt64ToInt:
    case EOpConvIntToInt64:
    case EOpConvUint64ToUint:
    case EOpConvUintToUint64:
    case EOpConvInt64ToUint64:
    case EOpConvUint64ToInt64:
    case EOpConvInt64ToUint:
    case EOpConvUintToInt64:
    case EOpConvUint64ToInt:
    case EOpConvIntToUint64:
#ifdef AMD_EXTENSIONS
    case EOpConvInt16ToBool:
    case EOpConvBoolToInt16:
    case EOpConvInt16ToInt:
    case EOpConvIntToInt16:
    case EOpConvInt16ToUint:
    case EOpConvUintToInt16:
    case EOpConvInt16ToInt64:
    case EOpConvInt64ToInt16:
    case EOpConvInt16ToUint64:
    case EOpConvUint64ToInt16:
    case EOpConvUint16ToBool:
    case EOpConvBoolToUint16:
    case EOpConvUint16ToInt:
    case EOpConvIntToUint16:
    case EOpConvUint16ToUint:
    case EOpConvUintToUint16:
    case EOpConvUint16ToInt64:
    case EOpConvInt64ToUint16:
    case EOpConvUint16ToUint64:
    case EOpConvUint64ToUint16:
    case EOpConvInt16ToUint16:
    case EOpConvUint16ToInt16:
#endif

    // unary operations
    case EOpNegative:
    case EOpLogicalNot:
    case EOpBitwiseNot:

    // binary operations
    case EOpAdd:
    case EOpSub:
    case EOpMul:
    case EOpVectorTimesScalar:
    case EOpDiv:
    case EOpMod:
    case EOpRightShift:
    case EOpLeftShift:
    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:
    case EOpLogicalOr:
    case EOpLogicalXor:
    case EOpLogicalAnd:
    case EOpEqual:
    case EOpNotEqual:
    case EOpLessThan:
    case EOpGreaterThan:
    case EOpLessThanEqual:
    case EOpGreaterThanEqual:
        return true;
    default:
        return false;
    }
}

////////////////////////////////////////////////////////////////
//
// Member functions of the nodes used for building the tree.
//
////////////////////////////////////////////////////////////////

//
// Say whether or not an operation node changes the value of a variable.
//
// Returns true if state is modified.
//
bool TIntermOperator::modifiesState() const
{
    switch (op) {
    case EOpPostIncrement:
    case EOpPostDecrement:
    case EOpPreIncrement:
    case EOpPreDecrement:
    case EOpAssign:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpMulAssign:
    case EOpVectorTimesMatrixAssign:
    case EOpVectorTimesScalarAssign:
    case EOpMatrixTimesScalarAssign:
    case EOpMatrixTimesMatrixAssign:
    case EOpDivAssign:
    case EOpModAssign:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
    case EOpLeftShiftAssign:
    case EOpRightShiftAssign:
        return true;
    default:
        return false;
    }
}

//
// returns true if the operator is for one of the constructors
//
bool TIntermOperator::isConstructor() const
{
    return op > EOpConstructGuardStart && op < EOpConstructGuardEnd;
}

//
// Make sure the type of an operator is appropriate for its
// combination of operation and operand type.  This will invoke
// promoteUnary, promoteBinary, etc as needed.
//
// Returns false if nothing makes sense.
//
bool TIntermediate::promote(TIntermOperator* node)
{
    if (node == nullptr)
        return false;

    if (node->getAsUnaryNode())
        return promoteUnary(*node->getAsUnaryNode());

    if (node->getAsBinaryNode())
        return promoteBinary(*node->getAsBinaryNode());

    if (node->getAsAggregate())
        return promoteAggregate(*node->getAsAggregate());

    return false;
}

//
// See TIntermediate::promote
//
bool TIntermediate::promoteUnary(TIntermUnary& node)
{
    const TOperator op    = node.getOp();
    TIntermTyped* operand = node.getOperand();

    switch (op) {
    case EOpLogicalNot:
        // Convert operand to a boolean type
        if (operand->getBasicType() != EbtBool) {
            // Add constructor to boolean type. If that fails, we can't do it, so return false.
            TIntermTyped* converted = convertToBasicType(op, EbtBool, operand);
            if (converted == nullptr)
                return false;

            // Use the result of converting the node to a bool.
            node.setOperand(operand = converted); // also updates stack variable
        }
        break;
    case EOpBitwiseNot:
        if (operand->getBasicType() != EbtInt &&
            operand->getBasicType() != EbtUint &&
#ifdef AMD_EXTENSIONS
            operand->getBasicType() != EbtInt16 &&
            operand->getBasicType() != EbtUint16 &&
#endif
            operand->getBasicType() != EbtInt64 &&
            operand->getBasicType() != EbtUint64)

            return false;
        break;
    case EOpNegative:
    case EOpPostIncrement:
    case EOpPostDecrement:
    case EOpPreIncrement:
    case EOpPreDecrement:
        if (operand->getBasicType() != EbtInt &&
            operand->getBasicType() != EbtUint &&
            operand->getBasicType() != EbtInt64 &&
            operand->getBasicType() != EbtUint64 &&
#ifdef AMD_EXTENSIONS
            operand->getBasicType() != EbtInt16 &&
            operand->getBasicType() != EbtUint16 &&
#endif
            operand->getBasicType() != EbtFloat &&
#ifdef AMD_EXTENSIONS
            operand->getBasicType() != EbtFloat16 &&
#endif
            operand->getBasicType() != EbtDouble)

            return false;
        break;

    default:
        if (operand->getBasicType() != EbtFloat)

            return false;
    }

    node.setType(operand->getType());
    node.getWritableType().getQualifier().makeTemporary();

    return true;
}

void TIntermUnary::updatePrecision()
{
#ifdef AMD_EXTENSIONS
    if (getBasicType() == EbtInt || getBasicType() == EbtUint || getBasicType() == EbtFloat || getBasicType() == EbtFloat16) {
#else
    if (getBasicType() == EbtInt || getBasicType() == EbtUint || getBasicType() == EbtFloat) {
#endif
        if (operand->getQualifier().precision > getQualifier().precision)
            getQualifier().precision = operand->getQualifier().precision;
    }
}

// If it is not already, convert this node to the given basic type.
TIntermTyped* TIntermediate::convertToBasicType(TOperator op, TBasicType basicType, TIntermTyped* node) const
{
    if (node == nullptr)
        return nullptr;

    // It's already this basic type: nothing needs to be done, so use the node directly.
    if (node->getBasicType() == basicType)
        return node;

    const TType& type = node->getType();
    const TType newType(basicType, type.getQualifier().storage,
                        type.getVectorSize(), type.getMatrixCols(), type.getMatrixRows(), type.isVector());

    // Add constructor to the right vectorness of the right type. If that fails, we can't do it, so return nullptr.
    return addConversion(op, newType, node);
}

//
// See TIntermediate::promote
//
bool TIntermediate::promoteBinary(TIntermBinary& node)
{
    TOperator     op    = node.getOp();
    TIntermTyped* left  = node.getLeft();
    TIntermTyped* right = node.getRight();

    // Arrays and structures have to be exact matches.
    if ((left->isArray() || right->isArray() || left->getBasicType() == EbtStruct || right->getBasicType() == EbtStruct)
        && left->getType() != right->getType())
        return false;

    // Base assumption:  just make the type the same as the left
    // operand.  Only deviations from this will be coded.
    node.setType(left->getType());
    node.getWritableType().getQualifier().clear();

    // Composite and opaque types don't having pending operator changes, e.g.,
    // array, structure, and samplers.  Just establish final type and correctness.
    if (left->isArray() || left->getBasicType() == EbtStruct || left->getBasicType() == EbtSampler) {
        switch (op) {
        case EOpEqual:
        case EOpNotEqual:
            if (left->getBasicType() == EbtSampler) {
                // can't compare samplers
                return false;
            } else {
                // Promote to conditional
                node.setType(TType(EbtBool));
            }

            return true;

        case EOpAssign:
            // Keep type from above

            return true;

        default:
            return false;
        }
    }

    //
    // We now have only scalars, vectors, and matrices to worry about.
    //

    // HLSL implicitly promotes bool -> int for numeric operations.
    // (Implicit conversions to make the operands match each other's types were already done.)
    if (getSource() == EShSourceHlsl &&
        (left->getBasicType() == EbtBool || right->getBasicType() == EbtBool)) {
        switch (op) {
        case EOpLessThan:
        case EOpGreaterThan:
        case EOpLessThanEqual:
        case EOpGreaterThanEqual:

        case EOpRightShift:
        case EOpLeftShift:

        case EOpMod:

        case EOpAnd:
        case EOpInclusiveOr:
        case EOpExclusiveOr:

        case EOpAdd:
        case EOpSub:
        case EOpDiv:
        case EOpMul:
            left = addConversion(op, TType(EbtInt, EvqTemporary, left->getVectorSize()), left);
            right = addConversion(op, TType(EbtInt, EvqTemporary, right->getVectorSize()), right);
            if (left == nullptr || right == nullptr)
                return false;
            node.setLeft(left);
            node.setRight(right);
            break;

        default:
            break;
        }
    }

    // Do general type checks against individual operands (comparing left and right is coming up, checking mixed shapes after that)
    switch (op) {
    case EOpLessThan:
    case EOpGreaterThan:
    case EOpLessThanEqual:
    case EOpGreaterThanEqual:
        // Relational comparisons need numeric types and will promote to scalar Boolean.
        if (left->getBasicType() == EbtBool)
            return false;

        node.setType(TType(EbtBool, EvqTemporary, left->getVectorSize()));
        break;

    case EOpEqual:
    case EOpNotEqual:
        if (getSource() == EShSourceHlsl) {
            const int resultWidth = std::max(left->getVectorSize(), right->getVectorSize());

            // In HLSL, == or != on vectors means component-wise comparison.
            if (resultWidth > 1) {
                op = (op == EOpEqual) ? EOpVectorEqual : EOpVectorNotEqual;
                node.setOp(op);
            }

            node.setType(TType(EbtBool, EvqTemporary, resultWidth));
        } else {
            // All the above comparisons result in a bool (but not the vector compares)
            node.setType(TType(EbtBool));
        }
        break;

    case EOpLogicalAnd:
    case EOpLogicalOr:
    case EOpLogicalXor:
        if (getSource() == EShSourceHlsl) {
            TIntermTyped* convertedL = convertToBasicType(op, EbtBool, left);
            TIntermTyped* convertedR = convertToBasicType(op, EbtBool, right);
            if (convertedL == nullptr || convertedR == nullptr)
                return false;
            node.setLeft(left = convertedL);   // also updates stack variable
            node.setRight(right = convertedR); // also updates stack variable
        } else {
            // logical ops operate only on scalar Booleans and will promote to scalar Boolean.
            if (left->getBasicType() != EbtBool || left->isVector() || left->isMatrix())
                return false;
        }

        node.setType(TType(EbtBool, EvqTemporary, left->getVectorSize()));

        break;

    case EOpRightShift:
    case EOpLeftShift:
    case EOpRightShiftAssign:
    case EOpLeftShiftAssign:

    case EOpMod:
    case EOpModAssign:

    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
        if (getSource() == EShSourceHlsl)
            break;

        // Check for integer-only operands.
        if ((left->getBasicType() != EbtInt &&  left->getBasicType() != EbtUint &&
#ifdef AMD_EXTENSIONS
             left->getBasicType() != EbtInt16 && left->getBasicType() != EbtUint16 &&
#endif
             left->getBasicType() != EbtInt64 && left->getBasicType() != EbtUint64) ||
            (right->getBasicType() != EbtInt && right->getBasicType() != EbtUint &&
#ifdef AMD_EXTENSIONS
             right->getBasicType() != EbtInt16 && right->getBasicType() != EbtUint16 &&
#endif
             right->getBasicType() != EbtInt64 && right->getBasicType() != EbtUint64))
            return false;
        if (left->isMatrix() || right->isMatrix())
            return false;

        break;

    case EOpAdd:
    case EOpSub:
    case EOpDiv:
    case EOpMul:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpMulAssign:
    case EOpDivAssign:
        // check for non-Boolean operands
        if (left->getBasicType() == EbtBool || right->getBasicType() == EbtBool)
            return false;

    default:
        break;
    }

    // Compare left and right, and finish with the cases where the operand types must match
    switch (op) {
    case EOpLessThan:
    case EOpGreaterThan:
    case EOpLessThanEqual:
    case EOpGreaterThanEqual:

    case EOpEqual:
    case EOpNotEqual:
    case EOpVectorEqual:
    case EOpVectorNotEqual:

    case EOpLogicalAnd:
    case EOpLogicalOr:
    case EOpLogicalXor:
        return left->getType() == right->getType();

    case EOpMod:
    case EOpModAssign:

    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:

    case EOpAdd:
    case EOpSub:
    case EOpDiv:

    case EOpAddAssign:
    case EOpSubAssign:
    case EOpDivAssign:
        // Quick out in case the types do match
        if (left->getType() == right->getType())
            return true;

        // Fall through

    case EOpMul:
    case EOpMulAssign:
        // At least the basic type has to match
        if (left->getBasicType() != right->getBasicType())
            return false;

    default:
        break;
    }

    // Finish handling the case, for all ops, where both operands are scalars.
    if (left->isScalar() && right->isScalar())
        return true;

    // Finish handling the case, for all ops, where there are two vectors of different sizes
    if (left->isVector() && right->isVector() && left->getVectorSize() != right->getVectorSize() && right->getVectorSize() > 1)
        return false;

    //
    // We now have a mix of scalars, vectors, or matrices, for non-relational operations.
    //

    // Can these two operands be combined, what is the resulting type?
    TBasicType basicType = left->getBasicType();
    switch (op) {
    case EOpMul:
        if (!left->isMatrix() && right->isMatrix()) {
            if (left->isVector()) {
                if (left->getVectorSize() != right->getMatrixRows())
                    return false;
                node.setOp(op = EOpVectorTimesMatrix);
                node.setType(TType(basicType, EvqTemporary, right->getMatrixCols()));
            } else {
                node.setOp(op = EOpMatrixTimesScalar);
                node.setType(TType(basicType, EvqTemporary, 0, right->getMatrixCols(), right->getMatrixRows()));
            }
        } else if (left->isMatrix() && !right->isMatrix()) {
            if (right->isVector()) {
                if (left->getMatrixCols() != right->getVectorSize())
                    return false;
                node.setOp(op = EOpMatrixTimesVector);
                node.setType(TType(basicType, EvqTemporary, left->getMatrixRows()));
            } else {
                node.setOp(op = EOpMatrixTimesScalar);
            }
        } else if (left->isMatrix() && right->isMatrix()) {
            if (left->getMatrixCols() != right->getMatrixRows())
                return false;
            node.setOp(op = EOpMatrixTimesMatrix);
            node.setType(TType(basicType, EvqTemporary, 0, right->getMatrixCols(), left->getMatrixRows()));
        } else if (! left->isMatrix() && ! right->isMatrix()) {
            if (left->isVector() && right->isVector()) {
                ; // leave as component product
            } else if (left->isVector() || right->isVector()) {
                node.setOp(op = EOpVectorTimesScalar);
                if (right->isVector())
                    node.setType(TType(basicType, EvqTemporary, right->getVectorSize()));
            }
        } else {
            return false;
        }
        break;
    case EOpMulAssign:
        if (! left->isMatrix() && right->isMatrix()) {
            if (left->isVector()) {
                if (left->getVectorSize() != right->getMatrixRows() || left->getVectorSize() != right->getMatrixCols())
                    return false;
                node.setOp(op = EOpVectorTimesMatrixAssign);
            } else {
                return false;
            }
        } else if (left->isMatrix() && !right->isMatrix()) {
            if (right->isVector()) {
                return false;
            } else {
                node.setOp(op = EOpMatrixTimesScalarAssign);
            }
        } else if (left->isMatrix() && right->isMatrix()) {
            if (left->getMatrixCols() != left->getMatrixRows() || left->getMatrixCols() != right->getMatrixCols() || left->getMatrixCols() != right->getMatrixRows())
                return false;
            node.setOp(op = EOpMatrixTimesMatrixAssign);
        } else if (!left->isMatrix() && !right->isMatrix()) {
            if (left->isVector() && right->isVector()) {
                // leave as component product
            } else if (left->isVector() || right->isVector()) {
                if (! left->isVector())
                    return false;
                node.setOp(op = EOpVectorTimesScalarAssign);
            }
        } else {
            return false;
        }
        break;

    case EOpRightShift:
    case EOpLeftShift:
    case EOpRightShiftAssign:
    case EOpLeftShiftAssign:
        if (right->isVector() && (! left->isVector() || right->getVectorSize() != left->getVectorSize()))
            return false;
        break;

    case EOpAssign:
        if (left->getVectorSize() != right->getVectorSize() || left->getMatrixCols() != right->getMatrixCols() || left->getMatrixRows() != right->getMatrixRows())
            return false;
        // fall through

    case EOpAdd:
    case EOpSub:
    case EOpDiv:
    case EOpMod:
    case EOpAnd:
    case EOpInclusiveOr:
    case EOpExclusiveOr:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpDivAssign:
    case EOpModAssign:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:

        if ((left->isMatrix() && right->isVector()) ||
            (left->isVector() && right->isMatrix()) ||
            left->getBasicType() != right->getBasicType())
            return false;
        if (left->isMatrix() && right->isMatrix() && (left->getMatrixCols() != right->getMatrixCols() || left->getMatrixRows() != right->getMatrixRows()))
            return false;
        if (left->isVector() && right->isVector() && left->getVectorSize() != right->getVectorSize())
            return false;
        if (right->isVector() || right->isMatrix()) {
            node.getWritableType().shallowCopy(right->getType());
            node.getWritableType().getQualifier().makeTemporary();
        }
        break;

    default:
        return false;
    }

    //
    // One more check for assignment.
    //
    switch (op) {
    // The resulting type has to match the left operand.
    case EOpAssign:
    case EOpAddAssign:
    case EOpSubAssign:
    case EOpMulAssign:
    case EOpDivAssign:
    case EOpModAssign:
    case EOpAndAssign:
    case EOpInclusiveOrAssign:
    case EOpExclusiveOrAssign:
    case EOpLeftShiftAssign:
    case EOpRightShiftAssign:
        if (node.getType() != left->getType())
            return false;
        break;
    default:
        break;
    }

    return true;
}

//
// See TIntermediate::promote
//
bool TIntermediate::promoteAggregate(TIntermAggregate& node)
{
    TOperator op = node.getOp();
    TIntermSequence& args = node.getSequence();
    const int numArgs = static_cast<int>(args.size());

    // Presently, only hlsl does intrinsic promotions.
    if (getSource() != EShSourceHlsl)
        return true;

    // set of opcodes that can be promoted in this manner.
    switch (op) {
    case EOpAtan:
    case EOpClamp:
    case EOpCross:
    case EOpDistance:
    case EOpDot:
    case EOpDst:
    case EOpFaceForward:
    // case EOpFindMSB: TODO:
    // case EOpFindLSB: TODO:
    case EOpFma:
    case EOpMod:
    case EOpFrexp:
    case EOpLdexp:
    case EOpMix:
    case EOpLit:
    case EOpMax:
    case EOpMin:
    case EOpModf:
    // case EOpGenMul: TODO:
    case EOpPow:
    case EOpReflect:
    case EOpRefract:
    // case EOpSinCos: TODO:
    case EOpSmoothStep:
    case EOpStep:
        break;
    default:
        return true;
    }

    // TODO: array and struct behavior

    // Try converting all nodes to the given node's type
    TIntermSequence convertedArgs(numArgs, nullptr);

    // Try to convert all types to the nonConvArg type.
    for (int nonConvArg = 0; nonConvArg < numArgs; ++nonConvArg) {
        // Try converting all args to this arg's type
        for (int convArg = 0; convArg < numArgs; ++convArg) {
            convertedArgs[convArg] = addConversion(op, args[nonConvArg]->getAsTyped()->getType(),
                                                   args[convArg]->getAsTyped());
        }

        // If we successfully converted all the args, use the result.
        if (std::all_of(convertedArgs.begin(), convertedArgs.end(),
                        [](const TIntermNode* node) { return node != nullptr; })) {

            std::swap(args, convertedArgs);
            return true;
        }
    }

    return false;
}

void TIntermBinary::updatePrecision()
{
#ifdef AMD_EXTENSIONS
    if (getBasicType() == EbtInt || getBasicType() == EbtUint || getBasicType() == EbtFloat || getBasicType() == EbtFloat16) {
#else
    if (getBasicType() == EbtInt || getBasicType() == EbtUint || getBasicType() == EbtFloat) {
#endif
        getQualifier().precision = std::max(right->getQualifier().precision, left->getQualifier().precision);
        if (getQualifier().precision != EpqNone) {
            left->propagatePrecision(getQualifier().precision);
            right->propagatePrecision(getQualifier().precision);
        }
    }
}

void TIntermTyped::propagatePrecision(TPrecisionQualifier newPrecision)
{
#ifdef AMD_EXTENSIONS
    if (getQualifier().precision != EpqNone || (getBasicType() != EbtInt && getBasicType() != EbtUint && getBasicType() != EbtFloat && getBasicType() != EbtFloat16))
#else
    if (getQualifier().precision != EpqNone || (getBasicType() != EbtInt && getBasicType() != EbtUint && getBasicType() != EbtFloat))
#endif
        return;

    getQualifier().precision = newPrecision;

    TIntermBinary* binaryNode = getAsBinaryNode();
    if (binaryNode) {
        binaryNode->getLeft()->propagatePrecision(newPrecision);
        binaryNode->getRight()->propagatePrecision(newPrecision);

        return;
    }

    TIntermUnary* unaryNode = getAsUnaryNode();
    if (unaryNode) {
        unaryNode->getOperand()->propagatePrecision(newPrecision);

        return;
    }

    TIntermAggregate* aggregateNode = getAsAggregate();
    if (aggregateNode) {
        TIntermSequence operands = aggregateNode->getSequence();
        for (unsigned int i = 0; i < operands.size(); ++i) {
            TIntermTyped* typedNode = operands[i]->getAsTyped();
            if (! typedNode)
                break;
            typedNode->propagatePrecision(newPrecision);
        }

        return;
    }

    TIntermSelection* selectionNode = getAsSelectionNode();
    if (selectionNode) {
        TIntermTyped* typedNode = selectionNode->getTrueBlock()->getAsTyped();
        if (typedNode) {
            typedNode->propagatePrecision(newPrecision);
            typedNode = selectionNode->getFalseBlock()->getAsTyped();
            if (typedNode)
                typedNode->propagatePrecision(newPrecision);
        }

        return;
    }
}

TIntermTyped* TIntermediate::promoteConstantUnion(TBasicType promoteTo, TIntermConstantUnion* node) const
{
    const TConstUnionArray& rightUnionArray = node->getConstArray();
    int size = node->getType().computeNumComponents();

    TConstUnionArray leftUnionArray(size);

    for (int i=0; i < size; i++) {
        switch (promoteTo) {
        case EbtFloat:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i] = rightUnionArray[i];
                break;
            default:
                return node;
            }
            break;
        case EbtDouble:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i] = rightUnionArray[i];
                break;
            default:
                return node;
            }
            break;
#ifdef AMD_EXTENSIONS
        case EbtFloat16:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setDConst(static_cast<double>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
            case EbtFloat16:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            default:
                return node;
            }
            break;
#endif
        case EbtInt:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            case EbtUint:
                leftUnionArray[i].setIConst(static_cast<int>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i].setIConst(static_cast<int>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i].setIConst(static_cast<int>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setIConst(static_cast<int>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i].setIConst(static_cast<int>(rightUnionArray[i].getDConst()));
                break;
            default:
                return node;
            }
            break;
        case EbtUint:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setUConst(static_cast<unsigned int>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            case EbtInt64:
                leftUnionArray[i].setUConst(static_cast<unsigned int>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i].setUConst(static_cast<unsigned int>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setUConst(static_cast<unsigned int>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i].setUConst(static_cast<unsigned int>(rightUnionArray[i].getDConst()));
                break;
            default:
                return node;
            }
            break;
        case EbtBool:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setBConst(rightUnionArray[i].getIConst() != 0);
                break;
            case EbtUint:
                leftUnionArray[i].setBConst(rightUnionArray[i].getUConst() != 0);
                break;
            case EbtInt64:
                leftUnionArray[i].setBConst(rightUnionArray[i].getI64Const() != 0);
                break;
            case EbtUint64:
                leftUnionArray[i].setBConst(rightUnionArray[i].getU64Const() != 0);
                break;
            case EbtBool:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i].setBConst(rightUnionArray[i].getDConst() != 0.0);
                break;
            default:
                return node;
            }
            break;
        case EbtInt64:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setI64Const(static_cast<long long>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i].setI64Const(static_cast<long long>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            case EbtUint64:
                leftUnionArray[i].setI64Const(static_cast<long long>(rightUnionArray[i].getU64Const()));
                break;
            case EbtBool:
                leftUnionArray[i].setI64Const(static_cast<long long>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i].setI64Const(static_cast<long long>(rightUnionArray[i].getDConst()));
                break;
            default:
                return node;
            }
            break;
        case EbtUint64:
            switch (node->getType().getBasicType()) {
            case EbtInt:
                leftUnionArray[i].setU64Const(static_cast<unsigned long long>(rightUnionArray[i].getIConst()));
                break;
            case EbtUint:
                leftUnionArray[i].setU64Const(static_cast<unsigned long long>(rightUnionArray[i].getUConst()));
                break;
            case EbtInt64:
                leftUnionArray[i].setU64Const(static_cast<unsigned long long>(rightUnionArray[i].getI64Const()));
                break;
            case EbtUint64:
                leftUnionArray[i] = rightUnionArray[i];
                break;
            case EbtBool:
                leftUnionArray[i].setU64Const(static_cast<unsigned long long>(rightUnionArray[i].getBConst()));
                break;
            case EbtFloat:
            case EbtDouble:
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
#endif
                leftUnionArray[i].setU64Const(static_cast<unsigned long long>(rightUnionArray[i].getDConst()));
                break;
            default:
                return node;
            }
            break;
        default:
            return node;
        }
    }

    const TType& t = node->getType();

    return addConstantUnion(leftUnionArray, TType(promoteTo, t.getQualifier().storage, t.getVectorSize(), t.getMatrixCols(), t.getMatrixRows()),
                            node->getLoc());
}

void TIntermAggregate::addToPragmaTable(const TPragmaTable& pTable)
{
    assert(!pragmaTable);
    pragmaTable = new TPragmaTable();
    *pragmaTable = pTable;
}

// If either node is a specialization constant, while the other is
// a constant (or specialization constant), the result is still
// a specialization constant.
bool TIntermediate::specConstantPropagates(const TIntermTyped& node1, const TIntermTyped& node2)
{
    return (node1.getType().getQualifier().isSpecConstant() && node2.getType().getQualifier().isConstant()) ||
           (node2.getType().getQualifier().isSpecConstant() && node1.getType().getQualifier().isConstant());
}

struct TextureUpgradeAndSamplerRemovalTransform : public TIntermTraverser {
    bool visitAggregate(TVisit, TIntermAggregate* ag) override {
        using namespace std;
        TIntermSequence& seq = ag->getSequence();
        // remove pure sampler variables
        TIntermSequence::iterator newEnd = remove_if(seq.begin(), seq.end(), [](TIntermNode* node) {
            TIntermSymbol* symbol = node->getAsSymbolNode();
            if (!symbol)
                return false;

            return (symbol->getBasicType() == EbtSampler && symbol->getType().getSampler().isPureSampler());
        });
        seq.erase(newEnd, seq.end());
        // replace constructors with sampler/textures
        // update textures into sampled textures
        for_each(seq.begin(), seq.end(), [](TIntermNode*& node) {
            TIntermSymbol* symbol = node->getAsSymbolNode();
            if (!symbol) {
                TIntermAggregate *constructor = node->getAsAggregate();
                if (constructor && constructor->getOp() == EOpConstructTextureSampler) {
                    if (!constructor->getSequence().empty())
                        node = constructor->getSequence()[0];
                }
            } else if (symbol->getBasicType() == EbtSampler && symbol->getType().getSampler().isTexture()) {
                symbol->getWritableType().getSampler().combined = true;
            }
        });
        return true;
    }
};

void TIntermediate::performTextureUpgradeAndSamplerRemovalTransformation(TIntermNode* root)
{
    TextureUpgradeAndSamplerRemovalTransform transform;
    root->traverse(&transform);
}

} // end namespace glslang
