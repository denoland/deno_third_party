"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var common_1 = require("../common");
exports.VariableDeclarationBase = base_1.ExclamationTokenableNode(base_1.TypedNode(base_1.InitializerExpressionableNode(base_1.BindingNamedNode(common_1.Node))));
var VariableDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(VariableDeclaration, _super);
    function VariableDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills this node with the specified structure.
     * @param structure - Structure to fill.
     */
    VariableDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.VariableDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes this variable declaration.
     */
    VariableDeclaration.prototype.remove = function () {
        var parent = this.getParentOrThrow();
        switch (parent.getKind()) {
            case typescript_1.SyntaxKind.VariableDeclarationList:
                removeFromDeclarationList(this);
                break;
            case typescript_1.SyntaxKind.CatchClause:
                removeFromCatchClause(this);
                break;
            default:
                throw new errors.NotImplementedError("Not implemented for syntax kind: " + parent.getKindName());
        }
        function removeFromDeclarationList(node) {
            var variableStatement = parent.getParentIfKindOrThrow(typescript_1.SyntaxKind.VariableStatement);
            var declarations = variableStatement.getDeclarations();
            if (declarations.length === 1)
                variableStatement.remove();
            else
                manipulation_1.removeCommaSeparatedChild(node);
        }
        function removeFromCatchClause(node) {
            manipulation_1.removeChildren({
                children: [
                    node.getPreviousSiblingIfKindOrThrow(typescript_1.SyntaxKind.OpenParenToken),
                    node,
                    node.getNextSiblingIfKindOrThrow(typescript_1.SyntaxKind.CloseParenToken)
                ],
                removePrecedingSpaces: true
            });
        }
    };
    return VariableDeclaration;
}(exports.VariableDeclarationBase));
exports.VariableDeclaration = VariableDeclaration;
