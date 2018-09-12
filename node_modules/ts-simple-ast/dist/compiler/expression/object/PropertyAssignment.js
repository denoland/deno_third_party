"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../../manipulation");
var typescript_1 = require("../../../typescript");
var utils_1 = require("../../../utils");
var base_1 = require("../../base");
var common_1 = require("../../common");
// This node only has a question token in order to tell the user about bad code.
// (See https://github.com/Microsoft/TypeScript/pull/5121/files)
exports.PropertyAssignmentBase = base_1.InitializerGetExpressionableNode(base_1.QuestionTokenableNode(base_1.PropertyNamedNode(common_1.Node)));
var PropertyAssignment = /** @class */ (function (_super) {
    tslib_1.__extends(PropertyAssignment, _super);
    function PropertyAssignment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Removes the initializer and returns the new shorthand property assignment.
     *
     * Note: The current node will no longer be valid because it's no longer a property assignment.
     */
    PropertyAssignment.prototype.removeInitializer = function () {
        var initializer = this.getInitializerOrThrow();
        var colonToken = initializer.getPreviousSiblingIfKindOrThrow(typescript_1.SyntaxKind.ColonToken);
        var childIndex = this.getChildIndex();
        var sourceFileText = this.sourceFile.getFullText();
        var insertPos = this.getStart();
        var newText = sourceFileText.substring(insertPos, colonToken.getPos()) + sourceFileText.substring(initializer.getEnd(), this.getEnd());
        var parent = this.getParentSyntaxList() || this.getParentOrThrow();
        manipulation_1.insertIntoParentTextRange({
            insertPos: insertPos,
            newText: newText,
            parent: parent,
            replacing: {
                textLength: this.getWidth()
            }
        });
        return parent.getChildAtIndexIfKindOrThrow(childIndex, typescript_1.SyntaxKind.ShorthandPropertyAssignment);
    };
    PropertyAssignment.prototype.setInitializer = function (textOrWriterFunction) {
        var initializer = this.getInitializerOrThrow();
        manipulation_1.insertIntoParentTextRange({
            insertPos: initializer.getStart(),
            newText: utils_1.getTextFromStringOrWriter(this.getWriterWithQueuedChildIndentation(), textOrWriterFunction),
            parent: this,
            replacing: {
                textLength: initializer.getWidth()
            }
        });
        return this;
    };
    /**
     * Removes this property.
     */
    PropertyAssignment.prototype.remove = function () {
        manipulation_1.removeCommaSeparatedChild(this);
    };
    return PropertyAssignment;
}(exports.PropertyAssignmentBase));
exports.PropertyAssignment = PropertyAssignment;
