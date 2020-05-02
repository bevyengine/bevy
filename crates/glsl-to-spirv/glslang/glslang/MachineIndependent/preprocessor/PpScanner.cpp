//
// Copyright (C) 2002-2005  3Dlabs Inc. Ltd.
// Copyright (C) 2013 LunarG, Inc.
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
/****************************************************************************\
Copyright (c) 2002, NVIDIA Corporation.

NVIDIA Corporation("NVIDIA") supplies this software to you in
consideration of your agreement to the following terms, and your use,
installation, modification or redistribution of this NVIDIA software
constitutes acceptance of these terms.  If you do not agree with these
terms, please do not use, install, modify or redistribute this NVIDIA
software.

In consideration of your agreement to abide by the following terms, and
subject to these terms, NVIDIA grants you a personal, non-exclusive
license, under NVIDIA's copyrights in this original NVIDIA software (the
"NVIDIA Software"), to use, reproduce, modify and redistribute the
NVIDIA Software, with or without modifications, in source and/or binary
forms; provided that if you redistribute the NVIDIA Software, you must
retain the copyright notice of NVIDIA, this notice and the following
text and disclaimers in all such redistributions of the NVIDIA Software.
Neither the name, trademarks, service marks nor logos of NVIDIA
Corporation may be used to endorse or promote products derived from the
NVIDIA Software without specific prior written permission from NVIDIA.
Except as expressly stated in this notice, no other rights or licenses
express or implied, are granted by NVIDIA herein, including but not
limited to any patent rights that may be infringed by your derivative
works or by other works in which the NVIDIA Software may be
incorporated. No hardware is licensed hereunder.

THE NVIDIA SOFTWARE IS BEING PROVIDED ON AN "AS IS" BASIS, WITHOUT
WARRANTIES OR CONDITIONS OF ANY KIND, EITHER EXPRESS OR IMPLIED,
INCLUDING WITHOUT LIMITATION, WARRANTIES OR CONDITIONS OF TITLE,
NON-INFRINGEMENT, MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, OR
ITS USE AND OPERATION EITHER ALONE OR IN COMBINATION WITH OTHER
PRODUCTS.

IN NO EVENT SHALL NVIDIA BE LIABLE FOR ANY SPECIAL, INDIRECT,
INCIDENTAL, EXEMPLARY, CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
TO, LOST PROFITS; PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF
USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) OR ARISING IN ANY WAY
OUT OF THE USE, REPRODUCTION, MODIFICATION AND/OR DISTRIBUTION OF THE
NVIDIA SOFTWARE, HOWEVER CAUSED AND WHETHER UNDER THEORY OF CONTRACT,
TORT (INCLUDING NEGLIGENCE), STRICT LIABILITY OR OTHERWISE, EVEN IF
NVIDIA HAS BEEN ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
\****************************************************************************/

#ifndef _CRT_SECURE_NO_WARNINGS
#define _CRT_SECURE_NO_WARNINGS
#endif

#include <cstdlib>
#include <cstring>

#include "PpContext.h"
#include "PpTokens.h"
#include "../Scan.h"

namespace glslang {

///////////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////// Floating point constants: /////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////

/*
* lFloatConst() - Scan a single- or double-precision floating point constant.  Assumes that the scanner
*         has seen at least one digit, followed by either a decimal '.' or the
*         letter 'e', or a precision ending (e.g., F or LF).
*/

int TPpContext::lFloatConst(int len, int ch, TPpToken* ppToken)
{
    bool HasDecimalOrExponent = false;
    int isDouble = 0;
    bool generateFloat16 = false;
    bool acceptFloat16 = parseContext.intermediate.getSource() == EShSourceHlsl;
    bool isFloat16 = false;
    bool requireHF = false;
#ifdef AMD_EXTENSIONS
    if (parseContext.extensionTurnedOn(E_GL_AMD_gpu_shader_half_float)) {
        acceptFloat16 = true;
        generateFloat16 = true;
        requireHF = true;
    }
#endif

    const auto saveName = [&](int ch) {
        if (len <= MaxTokenLength)
            ppToken->name[len++] = static_cast<char>(ch);
    };

    // Decimal:

    if (ch == '.') {
        HasDecimalOrExponent = true;
        saveName(ch);
        ch = getChar();

        // 1.#INF or -1.#INF
        if (ch == '#') {
            if ((len <  2) ||
                (len == 2 && ppToken->name[0] != '1') ||
                (len == 3 && ppToken->name[1] != '1' && !(ppToken->name[0] == '-' || ppToken->name[0] == '+')) ||
                (len >  3))
                parseContext.ppError(ppToken->loc, "unexpected use of", "#", "");
            else {
                // we have 1.# or -1.# or +1.#, check for 'INF'
                if ((ch = getChar()) != 'I' ||
                    (ch = getChar()) != 'N' ||
                    (ch = getChar()) != 'F')
                    parseContext.ppError(ppToken->loc, "expected 'INF'", "#", "");
                else {
                    // we have [+-].#INF, and we are targeting IEEE 754, so wrap it up:
                    saveName('I');
                    saveName('N');
                    saveName('F');
                    ppToken->name[len] = '\0';
                    if (ppToken->name[0] == '-')
                        ppToken->i64val = 0xfff0000000000000; // -Infinity
                    else
                        ppToken->i64val = 0x7ff0000000000000; // +Infinity
                    return PpAtomConstFloat;
                }
            }
        }

        while (ch >= '0' && ch <= '9') {
            saveName(ch);
            ch = getChar();
        }
    }

    // Exponent:

    if (ch == 'e' || ch == 'E') {
        HasDecimalOrExponent = true;
        saveName(ch);
        ch = getChar();
        if (ch == '+' || ch == '-') {
            saveName(ch);
            ch = getChar();
        }
        if (ch >= '0' && ch <= '9') {
            while (ch >= '0' && ch <= '9') {
                saveName(ch);
                ch = getChar();
            }
        } else {
            parseContext.ppError(ppToken->loc, "bad character in float exponent", "", "");
        }
    }

    // Suffix:

    if (ch == 'l' || ch == 'L') {
        parseContext.doubleCheck(ppToken->loc, "double floating-point suffix");
        if (! HasDecimalOrExponent)
            parseContext.ppError(ppToken->loc, "float literal needs a decimal point or exponent", "", "");
        int ch2 = getChar();
        if (ch2 != 'f' && ch2 != 'F') {
            ungetChar();
            ungetChar();
        } else {
            saveName(ch);
            saveName(ch2);
            isDouble = 1;
        }
    } else if (acceptFloat16 && (ch == 'h' || ch == 'H')) {
#ifdef AMD_EXTENSIONS
        if (generateFloat16)
            parseContext.float16Check(ppToken->loc, "half floating-point suffix");
#endif
        if (!HasDecimalOrExponent)
            parseContext.ppError(ppToken->loc, "float literal needs a decimal point or exponent", "", "");
        if (requireHF) {
            int ch2 = getChar();
            if (ch2 != 'f' && ch2 != 'F') {
                ungetChar();
                ungetChar();
            } else {
                saveName(ch);
                saveName(ch2);
                isFloat16 = generateFloat16;
            }
        } else {
            saveName(ch);
            isFloat16 = generateFloat16;
        }
    } else if (ch == 'f' || ch == 'F') {
        parseContext.profileRequires(ppToken->loc,  EEsProfile, 300, nullptr, "floating-point suffix");
        if (! parseContext.relaxedErrors())
            parseContext.profileRequires(ppToken->loc, ~EEsProfile, 120, nullptr, "floating-point suffix");
        if (! HasDecimalOrExponent)
            parseContext.ppError(ppToken->loc, "float literal needs a decimal point or exponent", "", "");
        saveName(ch);
    } else
        ungetChar();

    // Patch up the name, length, etc.

    if (len > MaxTokenLength) {
        len = MaxTokenLength;
        parseContext.ppError(ppToken->loc, "float literal too long", "", "");
    }
    ppToken->name[len] = '\0';

    // Get the numerical value
    ppToken->dval = strtod(ppToken->name, nullptr);

    // Return the right token type
    if (isDouble)
        return PpAtomConstDouble;
    else if (isFloat16)
        return PpAtomConstFloat16;
    else
        return PpAtomConstFloat;
}

// Recognize a character literal.
//
// The first ' has already been accepted, read the rest, through the closing '.
//
// Always returns PpAtomConstInt.
//
int TPpContext::characterLiteral(TPpToken* ppToken)
{
    ppToken->name[0] = 0;
    ppToken->ival = 0;

    if (parseContext.intermediate.getSource() != EShSourceHlsl) {
        // illegal, except in macro definition, for which case we report the character
        return '\'';
    }

    int ch = getChar();
    switch (ch) {
    case '\'':
        // As empty sequence:  ''
        parseContext.ppError(ppToken->loc, "unexpected", "\'", "");
        return PpAtomConstInt;
    case '\\':
        // As escape sequence:  '\XXX'
        switch (ch = getChar()) {
        case 'a':
            ppToken->ival = 7;
            break;
        case 'b':
            ppToken->ival = 8;
            break;
        case 't':
            ppToken->ival = 9;
            break;
        case 'n':
            ppToken->ival = 10;
            break;
        case 'v':
            ppToken->ival = 11;
            break;
        case 'f':
            ppToken->ival = 12;
            break;
        case 'r':
            ppToken->ival = 13;
            break;
        case 'x':
        case '0':
            parseContext.ppError(ppToken->loc, "octal and hex sequences not supported", "\\", "");
            break;
        default:
            // This catches '\'', '\"', '\?', etc.
            // Also, things like '\C' mean the same thing as 'C'
            // (after the above cases are filtered out).
            ppToken->ival = ch;
            break;
        }
        break;
    default:
        ppToken->ival = ch;
        break;
    }
    ppToken->name[0] = (char)ppToken->ival;
    ppToken->name[1] = '\0';
    ch = getChar();
    if (ch != '\'') {
        parseContext.ppError(ppToken->loc, "expected", "\'", "");
        // Look ahead for a closing '
        do {
            ch = getChar();
        } while (ch != '\'' && ch != EndOfInput && ch != '\n');
    }

    return PpAtomConstInt;
}

//
// Scanner used to tokenize source stream.
//
int TPpContext::tStringInput::scan(TPpToken* ppToken)
{
    int AlreadyComplained = 0;
    int len = 0;
    int ch = 0;
    int ii = 0;
    unsigned long long ival = 0;
    bool enableInt64 = pp->parseContext.version >= 450 && pp->parseContext.extensionTurnedOn(E_GL_ARB_gpu_shader_int64);
#ifdef AMD_EXTENSIONS
    bool enableInt16 = pp->parseContext.version >= 450 && pp->parseContext.extensionTurnedOn(E_GL_AMD_gpu_shader_int16);
#endif
    bool acceptHalf = pp->parseContext.intermediate.getSource() == EShSourceHlsl;
#ifdef AMD_EXTENSIONS
    if (pp->parseContext.extensionTurnedOn(E_GL_AMD_gpu_shader_half_float))
        acceptHalf = true;
#endif

    const auto floatingPointChar = [&](int ch) { return ch == '.' || ch == 'e' || ch == 'E' ||
                                                                     ch == 'f' || ch == 'F' ||
                                                     (acceptHalf && (ch == 'h' || ch == 'H')); };

    ppToken->ival = 0;
    ppToken->i64val = 0;
    ppToken->space = false;
    ch = getch();
    for (;;) {
        while (ch == ' ' || ch == '\t') {
            ppToken->space = true;
            ch = getch();
        }

        ppToken->loc = pp->parseContext.getCurrentLoc();
        len = 0;
        switch (ch) {
        default:
            // Single character token, including EndOfInput, '#' and '\' (escaped newlines are handled at a lower level, so this is just a '\' token)
            if (ch > PpAtomMaxSingle)
                ch = PpAtomBadToken;
            return ch;

        case 'A': case 'B': case 'C': case 'D': case 'E':
        case 'F': case 'G': case 'H': case 'I': case 'J':
        case 'K': case 'L': case 'M': case 'N': case 'O':
        case 'P': case 'Q': case 'R': case 'S': case 'T':
        case 'U': case 'V': case 'W': case 'X': case 'Y':
        case 'Z': case '_':
        case 'a': case 'b': case 'c': case 'd': case 'e':
        case 'f': case 'g': case 'h': case 'i': case 'j':
        case 'k': case 'l': case 'm': case 'n': case 'o':
        case 'p': case 'q': case 'r': case 's': case 't':
        case 'u': case 'v': case 'w': case 'x': case 'y':
        case 'z':
            do {
                if (len < MaxTokenLength) {
                    ppToken->name[len++] = (char)ch;
                    ch = getch();
                } else {
                    if (! AlreadyComplained) {
                        pp->parseContext.ppError(ppToken->loc, "name too long", "", "");
                        AlreadyComplained = 1;
                    }
                    ch = getch();
                }
            } while ((ch >= 'a' && ch <= 'z') ||
                     (ch >= 'A' && ch <= 'Z') ||
                     (ch >= '0' && ch <= '9') ||
                     ch == '_');

            // line continuation with no token before or after makes len == 0, and need to start over skipping white space, etc.
            if (len == 0)
                continue;

            ppToken->name[len] = '\0';
            ungetch();
            return PpAtomIdentifier;
        case '0':
            ppToken->name[len++] = (char)ch;
            ch = getch();
            if (ch == 'x' || ch == 'X') {
                // must be hexadecimal

                bool isUnsigned = false;
                bool isInt64 = false;
#ifdef AMD_EXTENSIONS
                bool isInt16 = false;
#endif
                ppToken->name[len++] = (char)ch;
                ch = getch();
                if ((ch >= '0' && ch <= '9') ||
                    (ch >= 'A' && ch <= 'F') ||
                    (ch >= 'a' && ch <= 'f')) {

                    ival = 0;
                    do {
                        if (ival <= 0x0fffffffu || (enableInt64 && ival <= 0x0fffffffffffffffull)) {
                            ppToken->name[len++] = (char)ch;
                            if (ch >= '0' && ch <= '9') {
                                ii = ch - '0';
                            } else if (ch >= 'A' && ch <= 'F') {
                                ii = ch - 'A' + 10;
                            } else if (ch >= 'a' && ch <= 'f') {
                                ii = ch - 'a' + 10;
                            } else
                                pp->parseContext.ppError(ppToken->loc, "bad digit in hexadecimal literal", "", "");
                            ival = (ival << 4) | ii;
                        } else {
                            if (! AlreadyComplained) {
                                pp->parseContext.ppError(ppToken->loc, "hexadecimal literal too big", "", "");
                                AlreadyComplained = 1;
                            }
                            ival = 0xffffffffffffffffull;
                        }
                        ch = getch();
                    } while ((ch >= '0' && ch <= '9') ||
                             (ch >= 'A' && ch <= 'F') ||
                             (ch >= 'a' && ch <= 'f'));
                } else {
                    pp->parseContext.ppError(ppToken->loc, "bad digit in hexadecimal literal", "", "");
                }
                if (ch == 'u' || ch == 'U') {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isUnsigned = true;

                    if (enableInt64) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 'l') || (ch == 'U' && nextCh == 'L')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt64 = true;
                        } else
                            ungetch();
                    }

#ifdef AMD_EXTENSIONS
                    if (enableInt16) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 's') || (ch == 'U' && nextCh == 'S')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt16 = true;
                        } else
                            ungetch();
                    }
#endif
                } else if (enableInt64 && (ch == 'l' || ch == 'L')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt64 = true;
#ifdef AMD_EXTENSIONS
                } else if (enableInt16 && (ch == 's' || ch == 'S')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt16 = true;
#endif
                } else
                    ungetch();
                ppToken->name[len] = '\0';

                if (isInt64) {
                    ppToken->i64val = ival;
                    return isUnsigned ? PpAtomConstUint64 : PpAtomConstInt64;
#ifdef AMD_EXTENSIONS
                } else if (isInt16) {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint16 : PpAtomConstInt16;
#endif
                } else {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint : PpAtomConstInt;
                }
            } else {
                // could be octal integer or floating point, speculative pursue octal until it must be floating point

                bool isUnsigned = false;
                bool isInt64 = false;
#ifdef AMD_EXTENSIONS
                bool isInt16 = false;
#endif
                bool octalOverflow = false;
                bool nonOctal = false;
                ival = 0;

                // see how much octal-like stuff we can read
                while (ch >= '0' && ch <= '7') {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    else if (! AlreadyComplained) {
                        pp->parseContext.ppError(ppToken->loc, "numeric literal too long", "", "");
                        AlreadyComplained = 1;
                    }
                    if (ival <= 0x1fffffffu || (enableInt64 && ival <= 0x1fffffffffffffffull)) {
                        ii = ch - '0';
                        ival = (ival << 3) | ii;
                    } else
                        octalOverflow = true;
                    ch = getch();
                }

                // could be part of a float...
                if (ch == '8' || ch == '9') {
                    nonOctal = true;
                    do {
                        if (len < MaxTokenLength)
                            ppToken->name[len++] = (char)ch;
                        else if (! AlreadyComplained) {
                            pp->parseContext.ppError(ppToken->loc, "numeric literal too long", "", "");
                            AlreadyComplained = 1;
                        }
                        ch = getch();
                    } while (ch >= '0' && ch <= '9');
                }
                if (floatingPointChar(ch))
                    return pp->lFloatConst(len, ch, ppToken);

                // wasn't a float, so must be octal...
                if (nonOctal)
                    pp->parseContext.ppError(ppToken->loc, "octal literal digit too large", "", "");

                if (ch == 'u' || ch == 'U') {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isUnsigned = true;

                    if (enableInt64) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 'l') || (ch == 'U' && nextCh == 'L')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt64 = true;
                        } else
                            ungetch();
                    }

#ifdef AMD_EXTENSIONS
                    if (enableInt16) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 's') || (ch == 'U' && nextCh == 'S')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt16 = true;
                        } else
                            ungetch();
                    }
#endif
                } else if (enableInt64 && (ch == 'l' || ch == 'L')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt64 = true;
#ifdef AMD_EXTENSIONS
                } else if (enableInt16 && (ch == 's' || ch == 'S')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt16 = true;
#endif
                } else
                    ungetch();
                ppToken->name[len] = '\0';

                if (octalOverflow)
                    pp->parseContext.ppError(ppToken->loc, "octal literal too big", "", "");

                if (isInt64) {
                    ppToken->i64val = ival;
                    return isUnsigned ? PpAtomConstUint64 : PpAtomConstInt64;
#ifdef AMD_EXTENSIONS
                } else if (isInt16) {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint16 : PpAtomConstInt16;
#endif
                } else {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint : PpAtomConstInt;
                }
            }
            break;
        case '1': case '2': case '3': case '4':
        case '5': case '6': case '7': case '8': case '9':
            // can't be hexadecimal or octal, is either decimal or floating point

            do {
                if (len < MaxTokenLength)
                    ppToken->name[len++] = (char)ch;
                else if (! AlreadyComplained) {
                    pp->parseContext.ppError(ppToken->loc, "numeric literal too long", "", "");
                    AlreadyComplained = 1;
                }
                ch = getch();
            } while (ch >= '0' && ch <= '9');
            if (floatingPointChar(ch))
                return pp->lFloatConst(len, ch, ppToken);
            else {
                // Finish handling signed and unsigned integers
                int numericLen = len;
                bool isUnsigned = false;
                bool isInt64 = false;
#ifdef AMD_EXTENSIONS
                bool isInt16 = false;
#endif
                if (ch == 'u' || ch == 'U') {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isUnsigned = true;

                    if (enableInt64) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 'l') || (ch == 'U' && nextCh == 'L')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt64 = true;
                        } else
                            ungetch();
                    }

#ifdef AMD_EXTENSIONS
                    if (enableInt16) {
                        int nextCh = getch();
                        if ((ch == 'u' && nextCh == 's') || (ch == 'U' && nextCh == 'S')) {
                            if (len < MaxTokenLength)
                                ppToken->name[len++] = (char)nextCh;
                            isInt16 = true;
                        } else
                            ungetch();
                    }
#endif
                } else if (enableInt64 && (ch == 'l' || ch == 'L')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt64 = true;
#ifdef AMD_EXTENSIONS
                } else if (enableInt16 && (ch == 's' || ch == 'S')) {
                    if (len < MaxTokenLength)
                        ppToken->name[len++] = (char)ch;
                    isInt16 = true;
#endif
                } else
                    ungetch();

                ppToken->name[len] = '\0';
                ival = 0;
                const unsigned oneTenthMaxInt  = 0xFFFFFFFFu / 10;
                const unsigned remainderMaxInt = 0xFFFFFFFFu - 10 * oneTenthMaxInt;
                const unsigned long long oneTenthMaxInt64  = 0xFFFFFFFFFFFFFFFFull / 10;
                const unsigned long long remainderMaxInt64 = 0xFFFFFFFFFFFFFFFFull - 10 * oneTenthMaxInt64;
#ifdef AMD_EXTENSIONS
                const unsigned short oneTenthMaxInt16  = 0xFFFFu / 10;
                const unsigned short remainderMaxInt16 = 0xFFFFu - 10 * oneTenthMaxInt16;
#endif
                for (int i = 0; i < numericLen; i++) {
                    ch = ppToken->name[i] - '0';
                    bool overflow = false;
                    if (isInt64)
                        overflow = (ival > oneTenthMaxInt64 || (ival == oneTenthMaxInt64 && (unsigned long long)ch > remainderMaxInt64));
#ifdef AMD_EXTENSIONS
                    else if (isInt16)
                        overflow = (ival > oneTenthMaxInt16 || (ival == oneTenthMaxInt16 && (unsigned short)ch > remainderMaxInt16));
#endif
                    else
                        overflow = (ival > oneTenthMaxInt || (ival == oneTenthMaxInt && (unsigned)ch > remainderMaxInt));
                    if (overflow) {
                        pp->parseContext.ppError(ppToken->loc, "numeric literal too big", "", "");
                        ival = 0xFFFFFFFFFFFFFFFFull;
                        break;
                    } else
                        ival = ival * 10 + ch;
                }

                if (isInt64) {
                    ppToken->i64val = ival;
                    return isUnsigned ? PpAtomConstUint64 : PpAtomConstInt64;
#ifdef AMD_EXTENSIONS
                } else if (isInt16) {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint16 : PpAtomConstInt16;
#endif
                } else {
                    ppToken->ival = (int)ival;
                    return isUnsigned ? PpAtomConstUint : PpAtomConstInt;
                }
            }
            break;
        case '-':
            ch = getch();
            if (ch == '-') {
                return PpAtomDecrement;
            } else if (ch == '=') {
                return PPAtomSubAssign;
            } else {
                ungetch();
                return '-';
            }
        case '+':
            ch = getch();
            if (ch == '+') {
                return PpAtomIncrement;
            } else if (ch == '=') {
                return PPAtomAddAssign;
            } else {
                ungetch();
                return '+';
            }
        case '*':
            ch = getch();
            if (ch == '=') {
                return PPAtomMulAssign;
            } else {
                ungetch();
                return '*';
            }
        case '%':
            ch = getch();
            if (ch == '=') {
                return PPAtomModAssign;
            } else {
                ungetch();
                return '%';
            }
        case '^':
            ch = getch();
            if (ch == '^') {
                return PpAtomXor;
            } else {
                if (ch == '=')
                    return PpAtomXorAssign;
                else{
                    ungetch();
                    return '^';
                }
            }

        case '=':
            ch = getch();
            if (ch == '=') {
                return PpAtomEQ;
            } else {
                ungetch();
                return '=';
            }
        case '!':
            ch = getch();
            if (ch == '=') {
                return PpAtomNE;
            } else {
                ungetch();
                return '!';
            }
        case '|':
            ch = getch();
            if (ch == '|') {
                return PpAtomOr;
            } else if (ch == '=') {
                return PpAtomOrAssign;
            } else {
                ungetch();
                return '|';
            }
        case '&':
            ch = getch();
            if (ch == '&') {
                return PpAtomAnd;
            } else if (ch == '=') {
                return PpAtomAndAssign;
            } else {
                ungetch();
                return '&';
            }
        case '<':
            ch = getch();
            if (ch == '<') {
                ch = getch();
                if (ch == '=')
                    return PpAtomLeftAssign;
                else {
                    ungetch();
                    return PpAtomLeft;
                }
            } else if (ch == '=') {
                return PpAtomLE;
            } else {
                ungetch();
                return '<';
            }
        case '>':
            ch = getch();
            if (ch == '>') {
                ch = getch();
                if (ch == '=')
                    return PpAtomRightAssign;
                else {
                    ungetch();
                    return PpAtomRight;
                }
            } else if (ch == '=') {
                return PpAtomGE;
            } else {
                ungetch();
                return '>';
            }
        case '.':
            ch = getch();
            if (ch >= '0' && ch <= '9') {
                ungetch();
                return pp->lFloatConst(0, '.', ppToken);
            } else {
                ungetch();
                return '.';
            }
        case '/':
            ch = getch();
            if (ch == '/') {
                pp->inComment = true;
                do {
                    ch = getch();
                } while (ch != '\n' && ch != EndOfInput);
                ppToken->space = true;
                pp->inComment = false;

                return ch;
            } else if (ch == '*') {
                ch = getch();
                do {
                    while (ch != '*') {
                        if (ch == EndOfInput) {
                            pp->parseContext.ppError(ppToken->loc, "End of input in comment", "comment", "");
                            return ch;
                        }
                        ch = getch();
                    }
                    ch = getch();
                    if (ch == EndOfInput) {
                        pp->parseContext.ppError(ppToken->loc, "End of input in comment", "comment", "");
                        return ch;
                    }
                } while (ch != '/');
                ppToken->space = true;
                // loop again to get the next token...
                break;
            } else if (ch == '=') {
                return PPAtomDivAssign;
            } else {
                ungetch();
                return '/';
            }
            break;
        case '\'':
            return pp->characterLiteral(ppToken);
        case '"':
            // TODO: If this gets enhanced to handle escape sequences, or
            // anything that is different than what #include needs, then
            // #include needs to use scanHeaderName() for this.
            ch = getch();
            while (ch != '"' && ch != '\n' && ch != EndOfInput) {
                if (len < MaxTokenLength) {
                    ppToken->name[len] = (char)ch;
                    len++;
                    ch = getch();
                } else
                    break;
            };
            ppToken->name[len] = '\0';
            if (ch != '"') {
                ungetch();
                pp->parseContext.ppError(ppToken->loc, "End of line in string", "string", "");
            }
            return PpAtomConstString;
        case ':':
            ch = getch();
            if (ch == ':')
                return PpAtomColonColon;
            ungetch();
            return ':';
        }

        ch = getch();
    }
}

//
// The main functional entry point into the preprocessor, which will
// scan the source strings to figure out and return the next processing token.
//
// Return the token, or EndOfInput when no more tokens.
//
int TPpContext::tokenize(TPpToken& ppToken)
{
    for(;;) {
        int token = scanToken(&ppToken);

        // Handle token-pasting logic
        token = tokenPaste(token, ppToken);

        if (token == EndOfInput) {
            missingEndifCheck();
            return EndOfInput;
        }
        if (token == '#') {
            if (previous_token == '\n') {
                token = readCPPline(&ppToken);
                if (token == EndOfInput) {
                    missingEndifCheck();
                    return EndOfInput;
                }
                continue;
            } else {
                parseContext.ppError(ppToken.loc, "preprocessor directive cannot be preceded by another token", "#", "");
                return EndOfInput;
            }
        }
        previous_token = token;

        if (token == '\n')
            continue;

        // expand macros
        if (token == PpAtomIdentifier && MacroExpand(&ppToken, false, true) != 0)
            continue;

        switch (token) {
        case PpAtomIdentifier:
        case PpAtomConstInt:
        case PpAtomConstUint:
        case PpAtomConstFloat:
        case PpAtomConstInt64:
        case PpAtomConstUint64:
#ifdef AMD_EXTENSIONS
        case PpAtomConstInt16:
        case PpAtomConstUint16:
#endif
        case PpAtomConstDouble:
#ifdef AMD_EXTENSIONS
        case PpAtomConstFloat16:
#endif
            if (ppToken.name[0] == '\0')
                continue;
            break;
        case PpAtomConstString:
            if (parseContext.intermediate.getSource() != EShSourceHlsl) {
                // HLSL allows string literals.
                parseContext.ppError(ppToken.loc, "string literals not supported", "\"\"", "");
                continue;
            }
            break;
        case '\'':
            parseContext.ppError(ppToken.loc, "character literals not supported", "\'", "");
            continue;
        default:
            strcpy(ppToken.name, atomStrings.getString(token));
            break;
        }

        return token;
    }
}

//
// Do all token-pasting related combining of two pasted tokens when getting a
// stream of tokens from a replacement list. Degenerates to no processing if a
// replacement list is not the source of the token stream.
//
int TPpContext::tokenPaste(int token, TPpToken& ppToken)
{
    // starting with ## is illegal, skip to next token
    if (token == PpAtomPaste) {
        parseContext.ppError(ppToken.loc, "unexpected location", "##", "");
        return scanToken(&ppToken);
    }

    int resultToken = token; // "foo" pasted with "35" is an identifier, not a number

    // ## can be chained, process all in the chain at once
    while (peekPasting()) {
        TPpToken pastedPpToken;

        // next token has to be ##
        token = scanToken(&pastedPpToken);
        assert(token == PpAtomPaste);

        // This covers end of macro expansion
        if (endOfReplacementList()) {
            parseContext.ppError(ppToken.loc, "unexpected location; end of replacement list", "##", "");
            break;
        }

        // get the token after the ##
        token = scanToken(&pastedPpToken);

        // This covers end of argument expansion
        if (token == tMarkerInput::marker) {
            parseContext.ppError(ppToken.loc, "unexpected location; end of argument", "##", "");
            break;
        }

        // get the token text
        switch (resultToken) {
        case PpAtomIdentifier:
            // already have the correct text in token.names
            break;
        case '=':
        case '!':
        case '-':
        case '~':
        case '+':
        case '*':
        case '/':
        case '%':
        case '<':
        case '>':
        case '|':
        case '^':
        case '&':
        case PpAtomRight:
        case PpAtomLeft:
        case PpAtomAnd:
        case PpAtomOr:
        case PpAtomXor:
            strcpy(ppToken.name, atomStrings.getString(resultToken));
            strcpy(pastedPpToken.name, atomStrings.getString(token));
            break;
        default:
            parseContext.ppError(ppToken.loc, "not supported for these tokens", "##", "");
            return resultToken;
        }

        // combine the tokens
        if (strlen(ppToken.name) + strlen(pastedPpToken.name) > MaxTokenLength) {
            parseContext.ppError(ppToken.loc, "combined tokens are too long", "##", "");
            return resultToken;
        }
        strncat(ppToken.name, pastedPpToken.name, MaxTokenLength - strlen(ppToken.name));

        // correct the kind of token we are making, if needed (identifiers stay identifiers)
        if (resultToken != PpAtomIdentifier) {
            int newToken = atomStrings.getAtom(ppToken.name);
            if (newToken > 0)
                resultToken = newToken;
            else
                parseContext.ppError(ppToken.loc, "combined token is invalid", "##", "");
        }
    }

    return resultToken;
}

// Checks if we've seen balanced #if...#endif
void TPpContext::missingEndifCheck()
{
    if (ifdepth > 0)
        parseContext.ppError(parseContext.getCurrentLoc(), "missing #endif", "", "");
}

} // end namespace glslang
