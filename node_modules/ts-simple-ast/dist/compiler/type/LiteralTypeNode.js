"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var LiteralTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(LiteralTypeNode, _super);
    function LiteralTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the literal type node's literal.
     */
    LiteralTypeNode.prototype.getLiteral = function () {
        // this statement is to be notified in case this changes
        var tsLiteral = this.compilerNode.literal;
        return this.getNodeFromCompilerNode(tsLiteral);
    };
    return LiteralTypeNode;
}(TypeNode_1.TypeNode));
exports.LiteralTypeNode = LiteralTypeNode;
