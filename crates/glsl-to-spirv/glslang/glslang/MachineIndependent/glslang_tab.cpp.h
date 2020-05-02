/* A Bison parser, made by GNU Bison 3.0.4.  */

/* Bison interface for Yacc-like parsers in C

   Copyright (C) 1984, 1989-1990, 2000-2015 Free Software Foundation, Inc.

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.  */

/* As a special exception, you may create a larger work that contains
   part or all of the Bison parser skeleton and distribute that work
   under terms of your choice, so long as that work isn't itself a
   parser generator using the skeleton or a modified version thereof
   as a parser skeleton.  Alternatively, if you modify or redistribute
   the parser skeleton itself, you may (at your option) remove this
   special exception, which will cause the skeleton and the resulting
   Bison output files to be licensed under the GNU General Public
   License without this special exception.

   This special exception was added by the Free Software Foundation in
   version 2.2 of Bison.  */

#ifndef YY_YY_MACHINEINDEPENDENT_GLSLANG_TAB_CPP_H_INCLUDED
# define YY_YY_MACHINEINDEPENDENT_GLSLANG_TAB_CPP_H_INCLUDED
/* Debug traces.  */
#ifndef YYDEBUG
# define YYDEBUG 1
#endif
#if YYDEBUG
extern int yydebug;
#endif

/* Token type.  */
#ifndef YYTOKENTYPE
# define YYTOKENTYPE
  enum yytokentype
  {
    ATTRIBUTE = 258,
    VARYING = 259,
    CONST = 260,
    BOOL = 261,
    FLOAT = 262,
    DOUBLE = 263,
    INT = 264,
    UINT = 265,
    INT64_T = 266,
    UINT64_T = 267,
    INT16_T = 268,
    UINT16_T = 269,
    FLOAT16_T = 270,
    BREAK = 271,
    CONTINUE = 272,
    DO = 273,
    ELSE = 274,
    FOR = 275,
    IF = 276,
    DISCARD = 277,
    RETURN = 278,
    SWITCH = 279,
    CASE = 280,
    DEFAULT = 281,
    SUBROUTINE = 282,
    BVEC2 = 283,
    BVEC3 = 284,
    BVEC4 = 285,
    IVEC2 = 286,
    IVEC3 = 287,
    IVEC4 = 288,
    I64VEC2 = 289,
    I64VEC3 = 290,
    I64VEC4 = 291,
    UVEC2 = 292,
    UVEC3 = 293,
    UVEC4 = 294,
    U64VEC2 = 295,
    U64VEC3 = 296,
    U64VEC4 = 297,
    VEC2 = 298,
    VEC3 = 299,
    VEC4 = 300,
    MAT2 = 301,
    MAT3 = 302,
    MAT4 = 303,
    CENTROID = 304,
    IN = 305,
    OUT = 306,
    INOUT = 307,
    UNIFORM = 308,
    PATCH = 309,
    SAMPLE = 310,
    BUFFER = 311,
    SHARED = 312,
    COHERENT = 313,
    VOLATILE = 314,
    RESTRICT = 315,
    READONLY = 316,
    WRITEONLY = 317,
    DVEC2 = 318,
    DVEC3 = 319,
    DVEC4 = 320,
    DMAT2 = 321,
    DMAT3 = 322,
    DMAT4 = 323,
    F16VEC2 = 324,
    F16VEC3 = 325,
    F16VEC4 = 326,
    F16MAT2 = 327,
    F16MAT3 = 328,
    F16MAT4 = 329,
    I16VEC2 = 330,
    I16VEC3 = 331,
    I16VEC4 = 332,
    U16VEC2 = 333,
    U16VEC3 = 334,
    U16VEC4 = 335,
    NOPERSPECTIVE = 336,
    FLAT = 337,
    SMOOTH = 338,
    LAYOUT = 339,
    __EXPLICITINTERPAMD = 340,
    MAT2X2 = 341,
    MAT2X3 = 342,
    MAT2X4 = 343,
    MAT3X2 = 344,
    MAT3X3 = 345,
    MAT3X4 = 346,
    MAT4X2 = 347,
    MAT4X3 = 348,
    MAT4X4 = 349,
    DMAT2X2 = 350,
    DMAT2X3 = 351,
    DMAT2X4 = 352,
    DMAT3X2 = 353,
    DMAT3X3 = 354,
    DMAT3X4 = 355,
    DMAT4X2 = 356,
    DMAT4X3 = 357,
    DMAT4X4 = 358,
    F16MAT2X2 = 359,
    F16MAT2X3 = 360,
    F16MAT2X4 = 361,
    F16MAT3X2 = 362,
    F16MAT3X3 = 363,
    F16MAT3X4 = 364,
    F16MAT4X2 = 365,
    F16MAT4X3 = 366,
    F16MAT4X4 = 367,
    ATOMIC_UINT = 368,
    SAMPLER1D = 369,
    SAMPLER2D = 370,
    SAMPLER3D = 371,
    SAMPLERCUBE = 372,
    SAMPLER1DSHADOW = 373,
    SAMPLER2DSHADOW = 374,
    SAMPLERCUBESHADOW = 375,
    SAMPLER1DARRAY = 376,
    SAMPLER2DARRAY = 377,
    SAMPLER1DARRAYSHADOW = 378,
    SAMPLER2DARRAYSHADOW = 379,
    ISAMPLER1D = 380,
    ISAMPLER2D = 381,
    ISAMPLER3D = 382,
    ISAMPLERCUBE = 383,
    ISAMPLER1DARRAY = 384,
    ISAMPLER2DARRAY = 385,
    USAMPLER1D = 386,
    USAMPLER2D = 387,
    USAMPLER3D = 388,
    USAMPLERCUBE = 389,
    USAMPLER1DARRAY = 390,
    USAMPLER2DARRAY = 391,
    SAMPLER2DRECT = 392,
    SAMPLER2DRECTSHADOW = 393,
    ISAMPLER2DRECT = 394,
    USAMPLER2DRECT = 395,
    SAMPLERBUFFER = 396,
    ISAMPLERBUFFER = 397,
    USAMPLERBUFFER = 398,
    SAMPLERCUBEARRAY = 399,
    SAMPLERCUBEARRAYSHADOW = 400,
    ISAMPLERCUBEARRAY = 401,
    USAMPLERCUBEARRAY = 402,
    SAMPLER2DMS = 403,
    ISAMPLER2DMS = 404,
    USAMPLER2DMS = 405,
    SAMPLER2DMSARRAY = 406,
    ISAMPLER2DMSARRAY = 407,
    USAMPLER2DMSARRAY = 408,
    SAMPLEREXTERNALOES = 409,
    SAMPLER = 410,
    SAMPLERSHADOW = 411,
    TEXTURE1D = 412,
    TEXTURE2D = 413,
    TEXTURE3D = 414,
    TEXTURECUBE = 415,
    TEXTURE1DARRAY = 416,
    TEXTURE2DARRAY = 417,
    ITEXTURE1D = 418,
    ITEXTURE2D = 419,
    ITEXTURE3D = 420,
    ITEXTURECUBE = 421,
    ITEXTURE1DARRAY = 422,
    ITEXTURE2DARRAY = 423,
    UTEXTURE1D = 424,
    UTEXTURE2D = 425,
    UTEXTURE3D = 426,
    UTEXTURECUBE = 427,
    UTEXTURE1DARRAY = 428,
    UTEXTURE2DARRAY = 429,
    TEXTURE2DRECT = 430,
    ITEXTURE2DRECT = 431,
    UTEXTURE2DRECT = 432,
    TEXTUREBUFFER = 433,
    ITEXTUREBUFFER = 434,
    UTEXTUREBUFFER = 435,
    TEXTURECUBEARRAY = 436,
    ITEXTURECUBEARRAY = 437,
    UTEXTURECUBEARRAY = 438,
    TEXTURE2DMS = 439,
    ITEXTURE2DMS = 440,
    UTEXTURE2DMS = 441,
    TEXTURE2DMSARRAY = 442,
    ITEXTURE2DMSARRAY = 443,
    UTEXTURE2DMSARRAY = 444,
    SUBPASSINPUT = 445,
    SUBPASSINPUTMS = 446,
    ISUBPASSINPUT = 447,
    ISUBPASSINPUTMS = 448,
    USUBPASSINPUT = 449,
    USUBPASSINPUTMS = 450,
    IMAGE1D = 451,
    IIMAGE1D = 452,
    UIMAGE1D = 453,
    IMAGE2D = 454,
    IIMAGE2D = 455,
    UIMAGE2D = 456,
    IMAGE3D = 457,
    IIMAGE3D = 458,
    UIMAGE3D = 459,
    IMAGE2DRECT = 460,
    IIMAGE2DRECT = 461,
    UIMAGE2DRECT = 462,
    IMAGECUBE = 463,
    IIMAGECUBE = 464,
    UIMAGECUBE = 465,
    IMAGEBUFFER = 466,
    IIMAGEBUFFER = 467,
    UIMAGEBUFFER = 468,
    IMAGE1DARRAY = 469,
    IIMAGE1DARRAY = 470,
    UIMAGE1DARRAY = 471,
    IMAGE2DARRAY = 472,
    IIMAGE2DARRAY = 473,
    UIMAGE2DARRAY = 474,
    IMAGECUBEARRAY = 475,
    IIMAGECUBEARRAY = 476,
    UIMAGECUBEARRAY = 477,
    IMAGE2DMS = 478,
    IIMAGE2DMS = 479,
    UIMAGE2DMS = 480,
    IMAGE2DMSARRAY = 481,
    IIMAGE2DMSARRAY = 482,
    UIMAGE2DMSARRAY = 483,
    STRUCT = 484,
    VOID = 485,
    WHILE = 486,
    IDENTIFIER = 487,
    TYPE_NAME = 488,
    FLOATCONSTANT = 489,
    DOUBLECONSTANT = 490,
    INTCONSTANT = 491,
    UINTCONSTANT = 492,
    INT64CONSTANT = 493,
    UINT64CONSTANT = 494,
    INT16CONSTANT = 495,
    UINT16CONSTANT = 496,
    BOOLCONSTANT = 497,
    FLOAT16CONSTANT = 498,
    LEFT_OP = 499,
    RIGHT_OP = 500,
    INC_OP = 501,
    DEC_OP = 502,
    LE_OP = 503,
    GE_OP = 504,
    EQ_OP = 505,
    NE_OP = 506,
    AND_OP = 507,
    OR_OP = 508,
    XOR_OP = 509,
    MUL_ASSIGN = 510,
    DIV_ASSIGN = 511,
    ADD_ASSIGN = 512,
    MOD_ASSIGN = 513,
    LEFT_ASSIGN = 514,
    RIGHT_ASSIGN = 515,
    AND_ASSIGN = 516,
    XOR_ASSIGN = 517,
    OR_ASSIGN = 518,
    SUB_ASSIGN = 519,
    LEFT_PAREN = 520,
    RIGHT_PAREN = 521,
    LEFT_BRACKET = 522,
    RIGHT_BRACKET = 523,
    LEFT_BRACE = 524,
    RIGHT_BRACE = 525,
    DOT = 526,
    COMMA = 527,
    COLON = 528,
    EQUAL = 529,
    SEMICOLON = 530,
    BANG = 531,
    DASH = 532,
    TILDE = 533,
    PLUS = 534,
    STAR = 535,
    SLASH = 536,
    PERCENT = 537,
    LEFT_ANGLE = 538,
    RIGHT_ANGLE = 539,
    VERTICAL_BAR = 540,
    CARET = 541,
    AMPERSAND = 542,
    QUESTION = 543,
    INVARIANT = 544,
    PRECISE = 545,
    HIGH_PRECISION = 546,
    MEDIUM_PRECISION = 547,
    LOW_PRECISION = 548,
    PRECISION = 549,
    PACKED = 550,
    RESOURCE = 551,
    SUPERP = 552
  };
#endif

/* Value type.  */
#if ! defined YYSTYPE && ! defined YYSTYPE_IS_DECLARED

union YYSTYPE
{
#line 68 "MachineIndependent/glslang.y" /* yacc.c:1909  */

    struct {
        glslang::TSourceLoc loc;
        union {
            glslang::TString *string;
            int i;
            unsigned int u;
            long long i64;
            unsigned long long u64;
            bool b;
            double d;
        };
        glslang::TSymbol* symbol;
    } lex;
    struct {
        glslang::TSourceLoc loc;
        glslang::TOperator op;
        union {
            TIntermNode* intermNode;
            glslang::TIntermNodePair nodePair;
            glslang::TIntermTyped* intermTypedNode;
        };
        union {
            glslang::TPublicType type;
            glslang::TFunction* function;
            glslang::TParameter param;
            glslang::TTypeLoc typeLine;
            glslang::TTypeList* typeList;
            glslang::TArraySizes* arraySizes;
            glslang::TIdentifierList* identifierList;
        };
    } interm;

#line 386 "MachineIndependent/glslang_tab.cpp.h" /* yacc.c:1909  */
};

typedef union YYSTYPE YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif



int yyparse (glslang::TParseContext* pParseContext);

#endif /* !YY_YY_MACHINEINDEPENDENT_GLSLANG_TAB_CPP_H_INCLUDED  */
