"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../../manipulation");
var expression_1 = require("../../expression");
var TaggedTemplateExpression = /** @class */ (function (_super) {
    tslib_1.__extends(TaggedTemplateExpression, _super);
    function TaggedTemplateExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the tag.
     */
    TaggedTemplateExpression.prototype.getTag = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.tag);
    };
    /**
     * Gets the template literal.
     */
    TaggedTemplateExpression.prototype.getTemplate = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.template);
    };
    /**
     * Removes the tag from the tagged template.
     * @returns The new template expression.
     */
    TaggedTemplateExpression.prototype.removeTag = function () {
        var parent = this.getParentSyntaxList() || this.getParentOrThrow();
        var index = this.getChildIndex();
        var template = this.getTemplate();
        manipulation_1.insertIntoParentTextRange({
            customMappings: function (newParent) { return [{ currentNode: template, newNode: newParent.getChildren()[index] }]; },
            parent: parent,
            insertPos: this.getStart(),
            newText: this.getTemplate().getText(),
            replacing: {
                textLength: this.getWidth(),
                nodes: [this]
            }
        });
        return parent.getChildAtIndex(index);
    };
    return TaggedTemplateExpression;
}(expression_1.MemberExpression));
exports.TaggedTemplateExpression = TaggedTemplateExpression;
