"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var expressioned_1 = require("./expressioned");
var MemberExpression_1 = require("./MemberExpression");
exports.ElementAccessExpressionBase = expressioned_1.LeftHandSideExpressionedNode(MemberExpression_1.MemberExpression);
var ElementAccessExpression = /** @class */ (function (_super) {
    tslib_1.__extends(ElementAccessExpression, _super);
    function ElementAccessExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this element access expression's argument expression or undefined if none exists.
     */
    ElementAccessExpression.prototype.getArgumentExpression = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.argumentExpression);
    };
    /**
     * Gets this element access expression's argument expression or throws if none exists.
     */
    ElementAccessExpression.prototype.getArgumentExpressionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getArgumentExpression(), "Expected to find an argument expression.");
    };
    return ElementAccessExpression;
}(exports.ElementAccessExpressionBase));
exports.ElementAccessExpression = ElementAccessExpression;
