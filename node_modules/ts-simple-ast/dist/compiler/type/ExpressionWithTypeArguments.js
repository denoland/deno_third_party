"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expression_1 = require("../expression");
var TypeNode_1 = require("./TypeNode");
exports.ExpressionWithTypeArgumentsBase = expression_1.LeftHandSideExpressionedNode(TypeNode_1.TypeNode);
var ExpressionWithTypeArguments = /** @class */ (function (_super) {
    tslib_1.__extends(ExpressionWithTypeArguments, _super);
    function ExpressionWithTypeArguments() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the type arguments.
     */
    ExpressionWithTypeArguments.prototype.getTypeArguments = function () {
        var _this = this;
        var typeArguments = this.compilerNode.typeArguments;
        if (typeArguments == null)
            return [];
        return typeArguments.map(function (a) { return _this.getNodeFromCompilerNode(a); });
    };
    return ExpressionWithTypeArguments;
}(exports.ExpressionWithTypeArgumentsBase));
exports.ExpressionWithTypeArguments = ExpressionWithTypeArguments;
