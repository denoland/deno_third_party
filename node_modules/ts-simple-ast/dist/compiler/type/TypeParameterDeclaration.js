"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var errors = require("../../errors");
var utils_1 = require("../../utils");
var base_1 = require("../base");
var common_1 = require("../common");
exports.TypeParameterDeclarationBase = base_1.NamedNode(common_1.Node);
var TypeParameterDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(TypeParameterDeclaration, _super);
    function TypeParameterDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the constraint node of the type parameter.
     * @deprecated - Use .getConstraint().
     */
    TypeParameterDeclaration.prototype.getConstraintNode = function () {
        return this.getConstraint();
    };
    /**
     * Gets the constraint of the type parameter.
     */
    TypeParameterDeclaration.prototype.getConstraint = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.constraint);
    };
    /**
     * Gets the constraint of the type parameter or throws if it doesn't exist.
     */
    TypeParameterDeclaration.prototype.getConstraintOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getConstraint(), "Expected to find the type parameter's constraint.");
    };
    /**
     * Sets the type parameter constraint.
     * @param text - Text to set as the constraint.
     */
    TypeParameterDeclaration.prototype.setConstraint = function (text) {
        if (utils_1.StringUtils.isNullOrWhitespace(text)) {
            this.removeConstraint();
            return this;
        }
        var constraint = this.getConstraint();
        if (constraint != null) {
            constraint.replaceWithText(text);
            return this;
        }
        var nameNode = this.getNameNode();
        manipulation_1.insertIntoParentTextRange({
            parent: this,
            insertPos: nameNode.getEnd(),
            newText: " extends " + text
        });
        return this;
    };
    /**
     * Removes the constraint type node.
     */
    TypeParameterDeclaration.prototype.removeConstraint = function () {
        removeConstraintOrDefault(this.getConstraint(), typescript_1.SyntaxKind.ExtendsKeyword);
        return this;
    };
    /**
     * Gets the default node of the type parameter.
     */
    TypeParameterDeclaration.prototype.getDefault = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.default);
    };
    /**
     * Gets the default node of the type parameter or throws if it doesn't exist.
     */
    TypeParameterDeclaration.prototype.getDefaultOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getDefault(), "Expected to find the type parameter's default.");
    };
    /**
     * Gets the default node of the type parameter.
     * @deprecated Use .getDefault().
     */
    TypeParameterDeclaration.prototype.getDefaultNode = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.default);
    };
    /**
     * Sets the type parameter default type node.
     * @param text - Text to set as the default type node.
     */
    TypeParameterDeclaration.prototype.setDefault = function (text) {
        if (utils_1.StringUtils.isNullOrWhitespace(text)) {
            this.removeDefault();
            return this;
        }
        var defaultNode = this.getDefault();
        if (defaultNode != null) {
            defaultNode.replaceWithText(text);
            return this;
        }
        var insertAfterNode = this.getConstraint() || this.getNameNode();
        manipulation_1.insertIntoParentTextRange({
            parent: this,
            insertPos: insertAfterNode.getEnd(),
            newText: " = " + text
        });
        return this;
    };
    /**
     * Removes the default type node.
     */
    TypeParameterDeclaration.prototype.removeDefault = function () {
        removeConstraintOrDefault(this.getDefault(), typescript_1.SyntaxKind.EqualsToken);
        return this;
    };
    /**
     * Removes this type parameter.
     */
    TypeParameterDeclaration.prototype.remove = function () {
        var parentSyntaxList = this.getParentSyntaxListOrThrow();
        var typeParameters = parentSyntaxList.getChildrenOfKind(typescript_1.SyntaxKind.TypeParameter);
        if (typeParameters.length === 1)
            removeAllTypeParameters();
        else
            manipulation_1.removeCommaSeparatedChild(this);
        function removeAllTypeParameters() {
            var children = [
                parentSyntaxList.getPreviousSiblingIfKindOrThrow(typescript_1.SyntaxKind.LessThanToken),
                parentSyntaxList,
                parentSyntaxList.getNextSiblingIfKindOrThrow(typescript_1.SyntaxKind.GreaterThanToken)
            ];
            manipulation_1.removeChildren({ children: children });
        }
    };
    return TypeParameterDeclaration;
}(exports.TypeParameterDeclarationBase));
exports.TypeParameterDeclaration = TypeParameterDeclaration;
function removeConstraintOrDefault(nodeToRemove, siblingKind) {
    if (nodeToRemove == null)
        return;
    manipulation_1.removeChildren({
        children: [nodeToRemove.getPreviousSiblingIfKindOrThrow(siblingKind), nodeToRemove],
        removePrecedingSpaces: true
    });
}
