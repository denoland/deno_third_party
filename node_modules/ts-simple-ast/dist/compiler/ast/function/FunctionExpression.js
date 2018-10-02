"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var expression_1 = require("../expression");
var statement_1 = require("../statement");
exports.FunctionExpressionBase = base_1.JSDocableNode(base_1.TextInsertableNode(base_1.BodiedNode(base_1.AsyncableNode(base_1.GeneratorableNode(statement_1.StatementedNode(base_1.TypeParameteredNode(base_1.SignaturedDeclaration(base_1.ModifierableNode(base_1.NameableNode(expression_1.PrimaryExpression))))))))));
var FunctionExpression = /** @class */ (function (_super) {
    tslib_1.__extends(FunctionExpression, _super);
    function FunctionExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return FunctionExpression;
}(exports.FunctionExpressionBase));
exports.FunctionExpression = FunctionExpression;
