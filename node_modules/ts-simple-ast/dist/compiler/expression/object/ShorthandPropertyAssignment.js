"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../../errors");
var manipulation_1 = require("../../../manipulation");
var typescript_1 = require("../../../typescript");
var base_1 = require("../../base");
var Node_1 = require("../../common/Node");
// This node only has an object assignment initializer, equals token, and question token, in order to tell the user about bad code
// (See https://github.com/Microsoft/TypeScript/pull/5121/files)
exports.ShorthandPropertyAssignmentBase = base_1.InitializerGetExpressionableNode(base_1.QuestionTokenableNode(base_1.NamedNode(Node_1.Node)));
var ShorthandPropertyAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(ShorthandPropertyAssignment, _super);
    function ShorthandPropertyAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets if the shorthand property assignment has an object assignment initializer.
     */
    ShorthandPropertyAssignment.prototype.hasObjectAssignmentInitializer = function () {
        return this.compilerNode.objectAssignmentInitializer != null;
    };
    /**
     * Gets the object assignment initializer or throws if it doesn't exist.
     */
    ShorthandPropertyAssignment.prototype.getObjectAssignmentInitializerOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getObjectAssignmentInitializer(), "Expected to find an object assignment initializer.");
    };
    /**
     * Gets the object assignment initializer if it exists.
     */
    ShorthandPropertyAssignment.prototype.getObjectAssignmentInitializer = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.objectAssignmentInitializer);
    };
    /**
     * Gets the equals token or throws if it doesn't exist.
     */
    ShorthandPropertyAssignment.prototype.getEqualsTokenOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getEqualsToken(), "Expected to find an equals token.");
    };
    /**
     * Gets the equals token if it exists.
     */
    ShorthandPropertyAssignment.prototype.getEqualsToken = function () {
        var equalsToken = this.compilerNode.equalsToken;
        if (equalsToken == null)
            return undefined;
        return this.getNodeFromCompilerNode(equalsToken);
    };
    /**
     * Remove the object assignment initializer.
     *
     * This is only useful to remove bad code.
     */
    ShorthandPropertyAssignment.prototype.removeObjectAssignmentInitializer = function () {
        if (!this.hasObjectAssignmentInitializer())
            return this;
        manipulation_1.removeChildren({
            children: [this.getEqualsTokenOrThrow(), this.getObjectAssignmentInitializerOrThrow()],
            removePrecedingSpaces: true
        });
        return this;
    };
    /**
     * Sets the initializer.
     *
     * Note: The current node will no longer be valid because it's no longer a shorthand property assignment.
     * @param text - New text to set for the initializer.
     */
    ShorthandPropertyAssignment.prototype.setInitializer = function (text) {
        var parent = this.getParentSyntaxList() || this.getParentOrThrow();
        var childIndex = this.getChildIndex();
        manipulation_1.insertIntoParentTextRange({
            insertPos: this.getStart(),
            newText: this.getText() + (": " + text),
            parent: parent,
            replacing: {
                textLength: this.getWidth()
            }
        });
        return parent.getChildAtIndexIfKindOrThrow(childIndex, typescript_1.SyntaxKind.PropertyAssignment);
    };
    /**
     * Removes this property.
     */
    ShorthandPropertyAssignment.prototype.remove = function () {
        manipulation_1.removeCommaSeparatedChild(this);
    };
    return ShorthandPropertyAssignment;
}(exports.ShorthandPropertyAssignmentBase));
exports.ShorthandPropertyAssignment = ShorthandPropertyAssignment;
