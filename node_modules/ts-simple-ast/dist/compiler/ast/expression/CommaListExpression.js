"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
exports.CommaListExpressionBase = Expression_1.Expression;
var CommaListExpression = /** @class */ (function (_super) {
    tslib_1.__extends(CommaListExpression, _super);
    function CommaListExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the elements.
     */
    CommaListExpression.prototype.getElements = function () {
        var _this = this;
        return this.compilerNode.elements.map(function (e) { return _this.getNodeFromCompilerNode(e); });
    };
    return CommaListExpression;
}(exports.CommaListExpressionBase));
exports.CommaListExpression = CommaListExpression;
