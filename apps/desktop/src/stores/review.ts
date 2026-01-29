import { create } from "zustand";

/**
 * Review action type for submitting a review
 */
export enum ReviewAction {
  Approve = "approve",
  RequestChanges = "request_changes",
  Comment = "comment",
}

/**
 * An inline comment on a specific line in a file
 */
export interface InlineComment {
  id: string;
  filePath: string;
  line: number;
  side: "left" | "right";
  body: string;
  createdAt: Date;
}

/**
 * Review state for a specific task
 */
export interface TaskReviewState {
  /** Set of file paths that have been viewed */
  viewedFiles: Set<string>;
  /** Inline comments keyed by `${filePath}:${side}:${line}` */
  inlineComments: Map<string, InlineComment>;
  /** Overall review body */
  reviewBody: string;
  /** Selected review action */
  reviewAction: ReviewAction;
}

/**
 * Store for managing code review state
 */
interface ReviewStore {
  /** Review state per task ID */
  taskReviews: Map<string, TaskReviewState>;

  /** Get or create review state for a task */
  getTaskReview: (taskId: string) => TaskReviewState;

  /** Mark a file as viewed */
  markFileAsViewed: (taskId: string, filePath: string) => void;

  /** Unmark a file as viewed */
  unmarkFileAsViewed: (taskId: string, filePath: string) => void;

  /** Toggle file viewed status */
  toggleFileViewed: (taskId: string, filePath: string) => void;

  /** Check if a file is viewed */
  isFileViewed: (taskId: string, filePath: string) => boolean;

  /** Add an inline comment */
  addInlineComment: (
    taskId: string,
    filePath: string,
    line: number,
    side: "left" | "right",
    body: string
  ) => void;

  /** Remove an inline comment */
  removeInlineComment: (taskId: string, commentId: string) => void;

  /** Update an inline comment */
  updateInlineComment: (taskId: string, commentId: string, body: string) => void;

  /** Get inline comments for a file */
  getFileComments: (taskId: string, filePath: string) => InlineComment[];

  /** Get all inline comments for a task */
  getAllComments: (taskId: string) => InlineComment[];

  /** Set the review body */
  setReviewBody: (taskId: string, body: string) => void;

  /** Set the review action */
  setReviewAction: (taskId: string, action: ReviewAction) => void;

  /** Clear review state for a task */
  clearTaskReview: (taskId: string) => void;

  /** Get the count of viewed files */
  getViewedFilesCount: (taskId: string) => number;
}

const createEmptyTaskReview = (): TaskReviewState => ({
  viewedFiles: new Set(),
  inlineComments: new Map(),
  reviewBody: "",
  reviewAction: ReviewAction.Comment,
});

const getCommentKey = (filePath: string, side: "left" | "right", line: number): string =>
  `${filePath}:${side}:${line}`;

export const useReviewStore = create<ReviewStore>((set, get) => ({
  taskReviews: new Map(),

  getTaskReview: (taskId: string) => {
    const { taskReviews } = get();
    // Return existing review or a default object without mutating state.
    // This prevents setState during render. State will be created lazily
    // when mutation functions (setReviewAction, setReviewBody, etc.) are called.
    return taskReviews.get(taskId) ?? createEmptyTaskReview();
  },

  markFileAsViewed: (taskId: string, filePath: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId) ?? createEmptyTaskReview();
      const viewedFiles = new Set(review.viewedFiles);
      viewedFiles.add(filePath);
      taskReviews.set(taskId, { ...review, viewedFiles });
      return { taskReviews };
    });
  },

  unmarkFileAsViewed: (taskId: string, filePath: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId);
      if (review) {
        const viewedFiles = new Set(review.viewedFiles);
        viewedFiles.delete(filePath);
        taskReviews.set(taskId, { ...review, viewedFiles });
      }
      return { taskReviews };
    });
  },

  toggleFileViewed: (taskId: string, filePath: string) => {
    const { isFileViewed, markFileAsViewed, unmarkFileAsViewed } = get();
    if (isFileViewed(taskId, filePath)) {
      unmarkFileAsViewed(taskId, filePath);
    } else {
      markFileAsViewed(taskId, filePath);
    }
  },

  isFileViewed: (taskId: string, filePath: string) => {
    const { taskReviews } = get();
    const review = taskReviews.get(taskId);
    return review?.viewedFiles.has(filePath) ?? false;
  },

  addInlineComment: (
    taskId: string,
    filePath: string,
    line: number,
    side: "left" | "right",
    body: string
  ) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId) ?? createEmptyTaskReview();
      const inlineComments = new Map(review.inlineComments);

      const comment: InlineComment = {
        id: crypto.randomUUID(),
        filePath,
        line,
        side,
        body,
        createdAt: new Date(),
      };

      const key = getCommentKey(filePath, side, line);
      inlineComments.set(key, comment);
      taskReviews.set(taskId, { ...review, inlineComments });
      return { taskReviews };
    });
  },

  removeInlineComment: (taskId: string, commentId: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId);
      if (review) {
        const inlineComments = new Map(review.inlineComments);
        for (const [key, comment] of inlineComments) {
          if (comment.id === commentId) {
            inlineComments.delete(key);
            break;
          }
        }
        taskReviews.set(taskId, { ...review, inlineComments });
      }
      return { taskReviews };
    });
  },

  updateInlineComment: (taskId: string, commentId: string, body: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId);
      if (review) {
        const inlineComments = new Map(review.inlineComments);
        for (const [key, comment] of inlineComments) {
          if (comment.id === commentId) {
            inlineComments.set(key, { ...comment, body });
            break;
          }
        }
        taskReviews.set(taskId, { ...review, inlineComments });
      }
      return { taskReviews };
    });
  },

  getFileComments: (taskId: string, filePath: string) => {
    const { taskReviews } = get();
    const review = taskReviews.get(taskId);
    if (!review) return [];
    return Array.from(review.inlineComments.values()).filter(
      (comment) => comment.filePath === filePath
    );
  },

  getAllComments: (taskId: string) => {
    const { taskReviews } = get();
    const review = taskReviews.get(taskId);
    if (!review) return [];
    return Array.from(review.inlineComments.values());
  },

  setReviewBody: (taskId: string, body: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId) ?? createEmptyTaskReview();
      taskReviews.set(taskId, { ...review, reviewBody: body });
      return { taskReviews };
    });
  },

  setReviewAction: (taskId: string, action: ReviewAction) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      const review = taskReviews.get(taskId) ?? createEmptyTaskReview();
      taskReviews.set(taskId, { ...review, reviewAction: action });
      return { taskReviews };
    });
  },

  clearTaskReview: (taskId: string) => {
    set((state) => {
      const taskReviews = new Map(state.taskReviews);
      taskReviews.delete(taskId);
      return { taskReviews };
    });
  },

  getViewedFilesCount: (taskId: string) => {
    const { taskReviews } = get();
    const review = taskReviews.get(taskId);
    return review?.viewedFiles.size ?? 0;
  },
}));
