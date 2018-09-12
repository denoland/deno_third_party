"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
function TypeArgumentedNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getTypeArguments = function () {
            var _this = this;
            if (this.compilerNode.typeArguments == null)
                return [];
            return this.compilerNode.typeArguments.map(function (a) { return _this.getNodeFromCompilerNode(a); });
        };
        class_1.prototype.addTypeArgument = function (argumentText) {
            return this.addTypeArguments([argumentText])[0];
        };
        class_1.prototype.addTypeArguments = function (argumentTexts) {
            return this.insertTypeArguments(this.getTypeArguments().length, argumentTexts);
        };
        class_1.prototype.insertTypeArgument = function (index, argumentText) {
            return this.insertTypeArguments(index, [argumentText])[0];
        };
        class_1.prototype.insertTypeArguments = function (index, argumentTexts) {
            if (utils_1.ArrayUtils.isNullOrEmpty(argumentTexts))
                return [];
            var typeArguments = this.getTypeArguments();
            index = manipulation_1.verifyAndGetIndex(index, typeArguments.length);
            if (typeArguments.length === 0) {
                var identifier = this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.Identifier);
                manipulation_1.insertIntoParentTextRange({
                    insertPos: identifier.getEnd(),
                    parent: this,
                    newText: "<" + argumentTexts.join(", ") + ">"
                });
            }
            else {
                manipulation_1.insertIntoCommaSeparatedNodes({
                    parent: this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.LessThanToken).getNextSiblingIfKindOrThrow(typescript_1.SyntaxKind.SyntaxList),
                    currentNodes: typeArguments,
                    insertIndex: index,
                    newText: argumentTexts.join(", ")
                });
            }
            return manipulation_1.getNodesToReturn(this.getTypeArguments(), index, argumentTexts.length);
        };
        class_1.prototype.removeTypeArgument = function (typeArgOrIndex) {
            var typeArguments = this.getTypeArguments();
            if (typeArguments.length === 0)
                throw new errors.InvalidOperationError("Cannot remove a type argument when none exist.");
            var typeArgToRemove = typeof typeArgOrIndex === "number" ? getTypeArgFromIndex(typeArgOrIndex) : typeArgOrIndex;
            if (typeArguments.length === 1) {
                var childSyntaxList = typeArguments[0].getParentSyntaxListOrThrow();
                manipulation_1.removeChildren({
                    children: [
                        childSyntaxList.getPreviousSiblingIfKindOrThrow(typescript_1.SyntaxKind.LessThanToken),
                        childSyntaxList,
                        childSyntaxList.getNextSiblingIfKindOrThrow(typescript_1.SyntaxKind.GreaterThanToken)
                    ]
                });
            }
            else
                manipulation_1.removeCommaSeparatedChild(typeArgToRemove);
            return this;
            function getTypeArgFromIndex(index) {
                return typeArguments[manipulation_1.verifyAndGetIndex(index, typeArguments.length - 1)];
            }
        };
        return class_1;
    }(Base));
}
exports.TypeArgumentedNode = TypeArgumentedNode;
